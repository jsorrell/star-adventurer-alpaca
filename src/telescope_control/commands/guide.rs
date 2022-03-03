use synscan::AutoGuideSpeed;
use tokio::time::Duration;

use crate::rotation_direction::RotationDirection;
use crate::telescope_control::star_adventurer::StarAdventurer;
use crate::util::*;

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
        Ok(self.settings.autoguide_speed.read().await.multiplier()
            * Degrees::from(*self.settings.tracking_rate.read().await))
    }

    /// Sets the current RightAscension movement rate offset for telescope guiding (degrees/sec).
    pub async fn set_guide_rate_ra(&self, rate: Degrees) -> AscomResult<()> {
        let tracking_rate_deg = Degrees::from(*self.settings.tracking_rate.read().await);
        let lowest_guide_rate = AutoGuideSpeed::Eighth.multiplier() * tracking_rate_deg;
        let highest_guide_rate = AutoGuideSpeed::One.multiplier() * tracking_rate_deg;
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
                let try_distance = (try_speed.multiplier() * tracking_rate_deg - rate).abs();
                if try_distance < distance {
                    (try_speed, try_distance)
                } else {
                    (closest, distance)
                }
            },
        );

        if *self.settings.autoguide_speed.read().await == best_speed {
            return Ok(());
        }

        self.connection.set_autoguide_speed(best_speed).await?;
        *self.settings.autoguide_speed.write().await = best_speed;
        Ok(())
    }

    /// True if this telescope is capable of software-pulsed guiding (via the PulseGuide(GuideDirections, Int32) method)
    pub async fn can_pulse_guide(&self) -> AscomResult<bool> {
        Ok(true)
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

        let guide_speed = self.settings.autoguide_speed.read().await.multiplier()
            * Degrees::from(*self.settings.tracking_rate.read().await);
        let guide_direction = guide_direction
            .using(
                self.settings
                    .observation_location
                    .read()
                    .await
                    .get_rotation_direction_key(),
            )
            .into();
        let guide_rate = MotionRate::new(guide_speed, guide_direction);

        let _completed = self
            .connection
            .pulse_guide(guide_rate, Duration::from_millis(duration as u64))
            .await?
            .await
            .unwrap()?;
        Ok(())
    }

    /// True if a PulseGuide(GuideDirections, Int32) command is in progress, False otherwise
    pub async fn is_pulse_guiding(&self) -> AscomResult<bool> {
        Ok(self.connection.is_guiding().await?)
    }
}
