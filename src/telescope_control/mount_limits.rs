use crate::astro_math;
use crate::astro_math::modulo;
use crate::telescope_control::slew_def::Slew;
use crate::tracking_direction::TrackingDirection;
use crate::util::*;

/// Defines the motion range limit of the mount in terms of mechanical hour angle
/// Valid mechanical hour angles are in the range east-west, usually crossing zero
#[derive(Debug, Clone, Copy)]
pub struct MountLimits {
    east: Hours, // Lower end of the valid ha range
    west: Hours, // Higher end -- may be above 24; always greater than east
}

impl MountLimits {
    /// east: 0, west: 24 for no limit
    pub fn new(east: Hours, west: Hours) -> Self {
        let east = astro_math::modulo(east, 24.);
        let west = astro_math::modulo(west, 24.);
        Self {
            east,
            west: east + astro_math::modulo(west - east, 24.),
        }
    }

    pub fn is_valid_ha(&self, ha: Hours) -> bool {
        let ha = self.niceify_ha(ha);
        (self.east..=self.west).contains(&ha)
    }

    /// In range values get modulo'd to a value between east and east+range
    /// Out of range values closer to the eastern side get modulo'd below east
    /// Out of range values closer to the western side get modulo'd above west
    /// East preferred if equal
    fn niceify_ha(&self, ha: Hours) -> Hours {
        let ha = modulo(ha, 24.);
        if ha < self.east {
            let hha = ha + 24.;
            if hha < self.west {
                hha
            } else {
                // Find closest valid frontier
                let east_dist = self.east - ha;
                let west_dist = hha - self.west;
                if east_dist <= west_dist {
                    ha
                } else {
                    hha
                }
            }
        } else {
            ha
        }
    }

    pub fn is_valid_slew(&self, start: Hours, slew: &Slew) -> bool {
        if 24. < slew.distance() {
            return false;
        }

        let start = self.niceify_ha(start);
        if !self.is_valid_ha(start) {
            // Allow slews only toward closest valid edge
            start < self.east && slew.direction() == TrackingDirection::WithTracking
                || self.west < start && slew.direction() == TrackingDirection::AgainstTracking
        } else {
            // Ensure the distance is in the valid range
            let valid_distance = match slew.direction() {
                TrackingDirection::WithTracking => self.west - start,
                TrackingDirection::AgainstTracking => start - self.east,
            };
            slew.distance() <= valid_distance
        }
    }
}
