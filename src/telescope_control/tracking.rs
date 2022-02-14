use crate::astro_math::Degrees;
use crate::telescope_control::StarAdventurer;
use crate::util::*;
use synscan::Direction;

impl StarAdventurer {
    /// True if the Tracking property can be changed, turning telescope sidereal tracking on and off.
    pub async fn can_set_tracking(&self) -> AscomResult<bool> {
        Ok(true)
    }

    #[inline]
    pub(crate) fn get_tracking_direction(latitude: Degrees) -> Direction {
        if Self::in_north(latitude) {
            Direction::Clockwise
        } else {
            Direction::CounterClockwise
        }
    }

    /// The right ascension tracking rate (arcseconds per second, default = 0.0)
    pub async fn get_ra_rate(&self) -> AscomResult<f64> {
        Ok(0.)
    }

    /// True if the RightAscensionRate property can be changed to provide offset tracking in the right ascension axis.
    pub async fn can_set_ra_rate(&self) -> AscomResult<bool> {
        Ok(false)
    }

    /// Sets the right ascension tracking rate (arcseconds per second)
    pub async fn set_ra_rate(&self, _rate: f64) -> AscomResult<()> {
        Err(AscomError::from_msg(
            AscomErrorType::PropertyOrMethodNotImplemented,
            "Setting RA tracking rate is not supported".to_string(),
        ))
    }

    /// The declination tracking rate (arcseconds per second, default = 0.0)
    pub async fn get_declination_rate(&self) -> AscomResult<f64> {
        Ok(0.)
    }

    /// True if the DeclinationRate property can be changed to provide offset tracking in the declination axis
    pub async fn can_set_declination_rate(&self) -> AscomResult<bool> {
        Ok(false)
    }

    /// Sets the declination tracking rate (arcseconds per second)
    pub async fn set_declination_rate(&self, _rate: f64) -> AscomResult<()> {
        Err(AscomError::from_msg(
            AscomErrorType::PropertyOrMethodNotImplemented,
            "Declination tracking not available".to_string(),
        ))
    }

    /// Returns an array of supported DriveRates values that describe the permissible values of the TrackingRate property for this telescope type.
    pub async fn get_tracking_rates(&self) -> AscomResult<Vec<TrackingRate>> {
        Ok(vec![
            TrackingRate::Sidereal,
            TrackingRate::Lunar,
            TrackingRate::Solar,
            TrackingRate::King,
        ])
    }

    /// The current tracking rate of the telescope's sidereal drive.
    pub async fn get_tracking_rate(&self) -> AscomResult<TrackingRate> {
        Ok(self.state.read().await.tracking_rate)
    }

    /// Sets the tracking rate of the telescope's sidereal drive
    pub async fn set_tracking_rate(&self, tracking_rate: TrackingRate) -> AscomResult<()> {
        let mut state = self.state.write().await;
        // No change needed
        if state.tracking_rate == tracking_rate {
            return Ok(());
        }

        state.tracking_rate = tracking_rate;

        if state.motor_state.is_tracking() {
            let current_motor_rate = state.motor_state.determine_motion_rate();
            let target_motor_state = state.motor_state.clone_without_guiding();

            // Change current tracking rate
            // Changing speed while moving is fine b/c it's at low speed
            self.driver
                .change_motor_rate(
                    current_motor_rate,
                    target_motor_state.determine_motion_rate(),
                )
                .await?;

            state.motor_state = target_motor_state;
        };

        Ok(())
    }

    /// Returns the state of the telescope's sidereal tracking drive.
    pub async fn is_tracking(&self) -> AscomResult<bool> {
        Ok(self.state.read().await.motor_state.is_tracking())
    }

    /// Sets the state of the telescope's sidereal tracking drive.
    /// TODO does setting tracking to true stop gotos?
    /// TODO Does it change what they'll do when the gotos are over?
    /// TODO Going with can only set it while not gotoing
    pub async fn set_is_tracking(&self, should_track: bool) -> AscomResult<()> {
        let mut state = self.state.write().await;
        // Nothing to do
        if state.motor_state.is_tracking() == should_track {
            return Ok(());
        }

        if state.motor_state.is_parked() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Can't start tracking while parked".to_string(),
            ));
        }

        if state.motor_state.is_slewing() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Can't start tracking while slewing".to_string(),
            ));
        }

        if state.motor_state.is_manually_moving_axis() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Can't start tracking while moving axis".to_string(),
            ));
        }

        let current_rate = state.motor_state.determine_motion_rate();

        let target_state = if should_track {
            MotorState::Moving(MovingState::Constant {
                state: ConstantMotionState::Tracking,
                guiding_state: GuidingState::Idle,
                motion_rate: state
                    .tracking_rate
                    .into_motion_rate(state.rotation_direction_key),
            })
        } else {
            MotorState::Stationary(StationaryState::Unparked(GuidingState::Idle))
        };

        self.driver
            .change_motor_rate(current_rate, target_state.determine_motion_rate())
            .await?;
        state.motor_state = target_state;
        Ok(())
    }
}
