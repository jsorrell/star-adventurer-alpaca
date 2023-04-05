use crate::consts::ALPACA_DATE_FMT;
use crate::telescope_control::StarAdventurer;
use ascom_alpaca::api::{
    AlignmentModeResponse, Axis, AxisRate, Device, DriveRate, EquatorialSystemResponse,
    PutPulseGuideDirection, SideOfPierResponse, Telescope, TelescopeSetSideOfPierRequestSideOfPier,
};
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use chrono::{DateTime, Utc};

#[async_trait::async_trait]
impl Device for StarAdventurer {
    fn static_name(&self) -> &str {
        "StarAdventurer"
    }

    fn unique_id(&self) -> &str {
        "f2d8e3a1-6c52-4d34-b475-e88056182f2b"
    }

    /* Action */
    async fn action(
        &self,
        action: String,
        parameters: String,
    ) -> ascom_alpaca::ASCOMResult<String> {
        match &*action {
            "pending_declination_slew" => {
                let change = self.get_pending_dec_change().await;
                Ok(change.to_string())
            }
            "complete_declination_slew" => {
                self.complete_dec_slew().await;
                Ok("".to_string())
            }
            "set_pier_side_after_manual_move" => {
                let pier_side = match &*parameters {
                    "east" => SideOfPierResponse::East,
                    "west" => SideOfPierResponse::West,
                    _ => {
                        return Err(ASCOMError::new(
                            ASCOMErrorCode::INVALID_VALUE,
                            format!("Unknown pier side: \"{}\"", parameters),
                        ))
                    }
                };
                self.set_pier_side_after_manual_move(pier_side).await;
                Ok("".to_string())
            }
            _ => Err(ASCOMError::new(
                ASCOMErrorCode::NOT_IMPLEMENTED,
                "Action not implemented".to_string(),
            )),
        }
    }

    /* Command */
    async fn command_blind(&self, _command: String, _raw: String) -> ASCOMResult<()> {
        Err(ASCOMError::new(
            ASCOMErrorCode::NOT_IMPLEMENTED,
            "Blind commands not accepted".to_string(),
        ))
    }

    async fn command_bool(&self, _command: String, _raw: String) -> ASCOMResult<bool> {
        Err(ASCOMError::new(
            ASCOMErrorCode::NOT_IMPLEMENTED,
            "Bool commands not accepted".to_string(),
        ))
    }

    async fn command_string(&self, _command: String, _raw: String) -> ASCOMResult<String> {
        Err(ASCOMError::new(
            ASCOMErrorCode::NOT_IMPLEMENTED,
            "String commands not accepted".to_string(),
        ))
    }

    /* Connected */
    async fn connected(&self) -> ASCOMResult<bool> {
        Ok(self.is_connected().await)
    }

    async fn set_connected(&self, connected: bool) -> ASCOMResult<()> {
        if connected {
            tracing::warn!("Connecting");
            self.connect().await
        } else {
            tracing::warn!("Disconnecting");
            self.disconnect().await
        }
    }

    async fn description(&self) -> ASCOMResult<String> {
        Ok("StarAdventurer".to_owned())
    }

    async fn driver_info(&self) -> ASCOMResult<String> {
        Ok("Rust ALPACA driver for Star Adventurer".to_owned())
    }

    async fn driver_version(&self) -> ASCOMResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_owned())
    }

    async fn interface_version(&self) -> ASCOMResult<i32> {
        Ok(3)
    }

    async fn name(&self) -> ASCOMResult<String> {
        Ok("StarAdventurer".to_owned())
    }

    async fn supported_actions(&self) -> ASCOMResult<Vec<String>> {
        Ok(vec![])
    }
}

#[async_trait::async_trait]
impl Telescope for StarAdventurer {
    async fn alignment_mode(&self) -> ASCOMResult<AlignmentModeResponse> {
        self.get_alignment_mode().await
    }

    async fn altitude(&self) -> ASCOMResult<f64> {
        self.get_altitude().await
    }

    async fn aperture_area(&self) -> ASCOMResult<f64> {
        self.get_aperture_area().await
    }

    async fn aperture_diameter(&self) -> ASCOMResult<f64> {
        self.get_aperture().await
    }

    async fn at_home(&self) -> ASCOMResult<bool> {
        self.is_home().await
    }

    async fn at_park(&self) -> ASCOMResult<bool> {
        self.is_parked().await
    }

    async fn azimuth(&self) -> ASCOMResult<f64> {
        self.get_azimuth().await
    }

    async fn can_find_home(&self) -> ASCOMResult<bool> {
        self.can_find_home().await
    }

    async fn can_park(&self) -> ASCOMResult<bool> {
        self.can_park().await
    }

    async fn can_pulse_guide(&self) -> ASCOMResult<bool> {
        self.can_pulse_guide().await
    }

    async fn can_set_declination_rate(&self) -> ASCOMResult<bool> {
        self.can_set_declination_rate().await
    }

    async fn can_set_guide_rates(&self) -> ASCOMResult<bool> {
        self.can_set_guide_rates().await
    }

    async fn can_set_park(&self) -> ASCOMResult<bool> {
        self.can_set_park_pos().await
    }

    async fn can_set_pier_side(&self) -> ASCOMResult<bool> {
        self.can_set_side_of_pier().await
    }

    async fn can_set_right_ascension_rate(&self) -> ASCOMResult<bool> {
        self.can_set_ra_rate().await
    }

    async fn can_set_tracking(&self) -> ASCOMResult<bool> {
        self.can_set_tracking().await
    }

    async fn can_slew(&self) -> ASCOMResult<bool> {
        self.can_slew().await
    }

    async fn can_slew_alt_az(&self) -> ASCOMResult<bool> {
        self.can_slew_alt_az().await
    }

    async fn can_slew_alt_az_async(&self) -> ASCOMResult<bool> {
        self.can_slew_alt_az_async().await
    }

    async fn can_slew_async(&self) -> ASCOMResult<bool> {
        self.can_slew_async().await
    }

    async fn can_sync(&self) -> ASCOMResult<bool> {
        self.can_sync().await
    }

    async fn can_sync_alt_az(&self) -> ASCOMResult<bool> {
        self.can_sync_alt_az().await
    }

    async fn can_unpark(&self) -> ASCOMResult<bool> {
        self.can_unpark().await
    }

    async fn declination(&self) -> ASCOMResult<f64> {
        self.get_dec().await
    }

    async fn declination_rate(&self) -> ASCOMResult<f64> {
        self.get_declination_rate().await
    }

    async fn set_declination_rate(&self, declination_rate: f64) -> ASCOMResult<()> {
        self.set_declination_rate(declination_rate).await
    }

    async fn does_refraction(&self) -> ASCOMResult<bool> {
        self.does_refraction().await
    }

    async fn set_does_refraction(&self, does_refraction: bool) -> ASCOMResult<()> {
        self.set_does_refraction(does_refraction).await
    }

    async fn equatorial_system(&self) -> ASCOMResult<EquatorialSystemResponse> {
        self.get_equatorial_system().await
    }

    async fn focal_length(&self) -> ASCOMResult<f64> {
        self.get_focal_length().await
    }

    async fn guide_rate_declination(&self) -> ASCOMResult<f64> {
        self.get_guide_rate_declination().await
    }

    async fn set_guide_rate_declination(&self, guide_rate_declination: f64) -> ASCOMResult<()> {
        self.set_guide_rate_declination(guide_rate_declination)
            .await
    }

    async fn guide_rate_right_ascension(&self) -> ASCOMResult<f64> {
        self.get_guide_rate_ra().await
    }

    async fn set_guide_rate_right_ascension(
        &self,
        guide_rate_right_ascension: f64,
    ) -> ASCOMResult<()> {
        self.set_guide_rate_ra(guide_rate_right_ascension).await
    }

    async fn is_pulse_guiding(&self) -> ASCOMResult<bool> {
        self.is_pulse_guiding().await
    }

    async fn right_ascension(&self) -> ASCOMResult<f64> {
        self.get_ra().await
    }

    async fn right_ascension_rate(&self) -> ASCOMResult<f64> {
        self.get_ra_rate().await
    }

    async fn set_right_ascension_rate(&self, right_ascension_rate: f64) -> ASCOMResult<()> {
        self.set_ra_rate(right_ascension_rate).await
    }

    async fn side_of_pier(&self) -> ASCOMResult<SideOfPierResponse> {
        self.get_side_of_pier().await
    }

    async fn set_side_of_pier(
        &self,
        side_of_pier: TelescopeSetSideOfPierRequestSideOfPier,
    ) -> ASCOMResult<()> {
        self.set_side_of_pier(side_of_pier).await
    }

    async fn sidereal_time(&self) -> ASCOMResult<f64> {
        self.get_sidereal_time().await
    }

    async fn site_elevation(&self) -> ASCOMResult<f64> {
        self.get_elevation().await
    }

    async fn set_site_elevation(&self, site_elevation: f64) -> ASCOMResult<()> {
        self.set_elevation(site_elevation).await
    }

    async fn site_latitude(&self) -> ASCOMResult<f64> {
        self.get_latitude().await
    }

    async fn set_site_latitude(&self, site_latitude: f64) -> ASCOMResult<()> {
        self.set_latitude(site_latitude).await
    }

    async fn site_longitude(&self) -> ASCOMResult<f64> {
        self.get_longitude().await
    }

    async fn set_site_longitude(&self, site_longitude: f64) -> ASCOMResult<()> {
        self.set_longitude(site_longitude).await
    }

    async fn slewing(&self) -> ASCOMResult<bool> {
        self.is_slewing().await
    }

    async fn slew_settle_time(&self) -> ASCOMResult<i32> {
        self.get_slew_settle_time().await.map(|x| x as i32)
    }

    async fn set_slew_settle_time(&self, slew_settle_time: i32) -> ASCOMResult<()> {
        if slew_settle_time < 0 {
            return Err(ASCOMError::new(
                ASCOMErrorCode::INVALID_VALUE,
                "Slew settle time must be nonegative".to_string(),
            ));
        }
        self.set_slew_settle_time(slew_settle_time as u32).await
    }

    async fn target_declination(&self) -> ASCOMResult<f64> {
        self.get_target_declination().await
    }

    async fn set_target_declination(&self, target_declination: f64) -> ASCOMResult<()> {
        self.set_target_dec(target_declination).await
    }

    async fn target_right_ascension(&self) -> ASCOMResult<f64> {
        self.get_target_ra().await
    }

    async fn set_target_right_ascension(&self, target_right_ascension: f64) -> ASCOMResult<()> {
        self.set_target_ra(target_right_ascension).await
    }

    async fn tracking(&self) -> ASCOMResult<bool> {
        self.is_tracking().await
    }

    async fn set_tracking(&self, tracking: bool) -> ASCOMResult<()> {
        self.set_is_tracking(tracking).await
    }

    async fn tracking_rate(&self) -> ASCOMResult<i32> {
        self.get_tracking_rate()
            .await
            .map(|drive_rate| drive_rate.into())
    }

    async fn set_tracking_rate(&self, tracking_rate: DriveRate) -> ASCOMResult<()> {
        match DriveRate::try_from(tracking_rate) {
            Ok(r) => self.set_tracking_rate(r).await,
            Err(_e) => Err(ASCOMError::new(
                ASCOMErrorCode::INVALID_VALUE,
                "Invalid Tracking Rate".to_string(),
            )),
        }
    }

    async fn tracking_rates(&self) -> ASCOMResult<Vec<DriveRate>> {
        self.get_tracking_rates().await
    }

    async fn utcdate(&self) -> ASCOMResult<String> {
        self.get_utc_date()
            .await
            .map(|d| d.format(ALPACA_DATE_FMT).to_string())
    }

    async fn set_utcdate(&self, utcdate: String) -> ASCOMResult<()> {
        let get_utc_date = move || {
            let t = DateTime::parse_from_str(&utcdate, ALPACA_DATE_FMT)?;
            let naive_time = t.naive_utc();
            Ok::<_, chrono::ParseError>(DateTime::<Utc>::from_utc(naive_time, Utc))
        };

        let d = match get_utc_date() {
            Ok(d) => d,
            Err(_e) => {
                return Err(ASCOMError::new(
                    ASCOMErrorCode::INVALID_VALUE,
                    "Date format is incorrect".to_string(),
                ))
            }
        };

        self.set_utc_date(d).await
    }

    async fn abort_slew(&self) -> ASCOMResult<()> {
        self.abort_slew().await
    }

    async fn axis_rates(&self, axis: Axis) -> ASCOMResult<Vec<AxisRate>> {
        self.get_axis_rates(axis).await
    }

    async fn can_move_axis(&self, axis: Axis) -> ASCOMResult<bool> {
        self.can_move_axis(axis).await
    }

    async fn destination_side_of_pier(
        &self,
        right_ascension: f64,
        declination: f64,
    ) -> ASCOMResult<SideOfPierResponse> {
        self.predict_destination_side_of_pier(right_ascension, declination)
            .await
    }

    async fn find_home(&self) -> ASCOMResult<()> {
        self.find_home().await
    }

    async fn move_axis(&self, axis: Axis, rate: f64) -> ASCOMResult<()> {
        let result = self.move_axis(axis, rate).await;
        result
    }

    async fn park(&self) -> ASCOMResult<()> {
        self.park().await
    }

    async fn pulse_guide(
        &self,
        direction: PutPulseGuideDirection,
        duration: i32,
    ) -> ASCOMResult<()> {
        self.pulse_guide(direction, duration as u32).await
    }

    async fn set_park(&self) -> ASCOMResult<()> {
        self.set_park_pos().await
    }

    async fn slew_to_alt_az(&self, azimuth: f64, altitude: f64) -> ASCOMResult<()> {
        self.slew_to_alt_az(altitude, azimuth).await?;
        Ok(())
    }

    async fn slew_to_alt_az_async(&self, azimuth: f64, altitude: f64) -> ASCOMResult<()> {
        let _finish = self.slew_to_alt_az_async(altitude, azimuth).await?;
        Ok(())
    }

    async fn slew_to_coordinates(&self, right_ascension: f64, declination: f64) -> ASCOMResult<()> {
        self.slew_to_coordinates(right_ascension, declination)
            .await?;
        Ok(())
    }

    async fn slew_to_coordinates_async(
        &self,
        right_ascension: f64,
        declination: f64,
    ) -> ASCOMResult<()> {
        let _finish = self
            .slew_to_coordinates_async(right_ascension, declination)
            .await?;
        Ok(())
    }

    async fn slew_to_target(&self) -> ASCOMResult<()> {
        self.slew_to_target().await?;
        Ok(())
    }

    async fn slew_to_target_async(&self) -> ASCOMResult<()> {
        let _finish = self.slew_to_target_async().await?;
        Ok(())
    }

    async fn sync_to_alt_az(&self, azimuth: f64, altitude: f64) -> ASCOMResult<()> {
        self.sync_to_alt_az(altitude, azimuth).await
    }

    async fn sync_to_coordinates(&self, right_ascension: f64, declination: f64) -> ASCOMResult<()> {
        self.sync_to_coordinates(right_ascension, declination).await
    }

    async fn sync_to_target(&self) -> ASCOMResult<()> {
        self.sync_to_target().await
    }

    async fn unpark(&self) -> ASCOMResult<()> {
        self.unpark().await
    }
}
