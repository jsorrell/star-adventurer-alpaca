use crate::rotation_direction::RotationDirection;
use crate::telescope_control::driver::Driver;
use crate::telescope_control::{StarAdventurer, StateArc};
use crate::util::*;
use synscan::AutoGuideSpeed;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;
use tokio::task;
use tokio::time::{sleep_until, Duration, Instant};

impl StarAdventurer {
    /// True if the guide rate properties used for PulseGuide(GuideDirections, Int32) can ba adjusted.
    pub async fn can_set_guide_rates(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// The current Declination movement rate offset for telescope guiding (degrees/sec)
    pub async fn get_guide_rate_declination(&self) -> AscomResult<Degrees> {
        Ok(0.)
    }

    /// Sets the current Declination movement rate offset for telescope guiding (degrees/sec).
    pub async fn set_guide_rate_declination(&self, _rate: Degrees) -> AscomResult<()> {
        // This must "function" per ASCOM specs
        Ok(())
    }

    /// The current RightAscension movement rate offset for telescope guiding (degrees/sec)
    pub async fn get_guide_rate_ra(&self) -> AscomResult<Degrees> {
        let state = self.state.read().await;
        Ok(state.autoguide_speed.multiplier() * Degrees::from(state.tracking_rate))
    }

    /// Sets the current RightAscension movement rate offset for telescope guiding (degrees/sec).
    pub async fn set_guide_rate_ra(&self, rate: Degrees) -> AscomResult<()> {
        let mut state = self.state.write().await;
        let lowest_guide_rate =
            AutoGuideSpeed::Eighth.multiplier() * Degrees::from(state.tracking_rate);
        let highest_guide_rate =
            AutoGuideSpeed::One.multiplier() * Degrees::from(state.tracking_rate);
        if rate < lowest_guide_rate * 0.9 || highest_guide_rate * 1.1 < rate {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidValue,
                format!(
                    "Guide rate must be between {} and {}",
                    lowest_guide_rate, highest_guide_rate
                ),
            ));
        }

        let (best_speed, _distance) = [
            AutoGuideSpeed::Eighth,
            AutoGuideSpeed::Quarter,
            AutoGuideSpeed::Half,
            AutoGuideSpeed::ThreeQuarters,
            AutoGuideSpeed::One,
        ]
        .into_iter()
        .fold(
            (AutoGuideSpeed::Eighth, 99999.),
            |(closest, distance), try_speed| {
                let try_distance =
                    (try_speed.multiplier() * Degrees::from(state.tracking_rate) - rate).abs();
                if try_distance < distance {
                    (try_speed, try_distance)
                } else {
                    (closest, distance)
                }
            },
        );

        if state.autoguide_speed == best_speed {
            return Ok(());
        }

        // change speed
        state.autoguide_speed = best_speed;

        if state.motor_state.is_guiding() {
            let current_motion_rate = state.motor_state.determine_motion_rate();
            let target_motion_rate = state
                .motor_state
                .clone_without_guiding()
                .determine_motion_rate();

            if let Err(_e) = self
                .driver
                .change_motor_rate(current_motion_rate, target_motion_rate)
                .await
            {
                panic!("Fatal: Entered unknown state trying to cancel pulse guide");
            }
        }

        self.driver.set_autoguide_speed(best_speed).await?;
        Ok(())
    }

    /// True if this telescope is capable of software-pulsed guiding (via the PulseGuide(GuideDirections, Int32) method)
    pub async fn can_pulse_guide(&self) -> AscomResult<bool> {
        Ok(true)
    }

    async fn guide_task(
        state: StateArc,
        driver: Driver,
        end_time: Instant,
        cancel_rx: Receiver<()>,
    ) {
        let sleep = sleep_until(end_time);

        let end_guide = async {
            let mut state = state.write().await;

            let current_rate = state.motor_state.determine_motion_rate();
            let target_state = state.motor_state.clone_without_guiding();
            let target_rate = target_state.determine_motion_rate();

            if let Err(_e) = driver.change_motor_rate(current_rate, target_rate).await {
                panic!("Fatal: Entered unknown state trying to end pulse guide");
            }
            state.motor_state = target_state;
        };

        tokio::select! {
            _ = sleep => end_guide.await,
            _ = cancel_rx => log::info!("Guide task cancelled"),
        }
    }

    /// Moves the scope in the given direction for the given interval or time at the rate given by the corresponding guide rate property
    /// Synchronous b/c only one axis is guideable
    pub async fn pulse_guide(
        &self,
        guide_direction: GuideDirection,
        duration: u32,
    ) -> AscomResult<()> {
        if guide_direction == GuideDirection::North || guide_direction == GuideDirection::South {
            return Err(AscomError::from_msg(
                AscomErrorType::PropertyOrMethodNotImplemented,
                "Can't guide in declination".to_string(),
            ));
        }

        let mut state = self.state.write().await;

        if state.motor_state.is_parked() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Can't guide while parked".to_string(),
            ));
        }

        if state.is_slewing() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Can't guide while slewing".to_string(),
            ));
        }

        if state.motor_state.is_guiding() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Already guiding".to_string(),
            ));
        }

        let guide_speed = state.autoguide_speed.multiplier() * Degrees::from(state.tracking_rate);
        let guide_direction = guide_direction.using(state.rotation_direction_key).into();
        let guide_rate = MotionRate::new(guide_speed, guide_direction);

        let current_motion_rate = state.motor_state.determine_motion_rate();
        let target_motion_rate = current_motion_rate.clone() + guide_rate;

        // Start guide
        self.driver
            .change_motor_rate(current_motion_rate, target_motion_rate)
            .await?;
        let end_time = Instant::now() + Duration::from_millis(duration as u64);

        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

        /* Start task that will stop guiding when it's done */
        let guide_task = task::spawn(Self::guide_task(
            self.state.clone(),
            self.driver.clone(),
            end_time,
            cancel_rx,
        ));

        state
            .motor_state
            .pulse_guide(cancel_tx, target_motion_rate)
            .expect("Motor state is not pulse-guideable");
        std::mem::drop(state);
        guide_task.await.unwrap();
        Ok(())
    }

    /// True if a PulseGuide(GuideDirections, Int32) command is in progress, False otherwise
    pub async fn is_pulse_guiding(&self) -> AscomResult<bool> {
        let state = self.state.read().await;
        Ok(state.motor_state.is_guiding())
    }
}
