#![allow(dead_code)]
use chrono::{Datelike, Timelike};
use polynomials::poly;
use std::f64::consts::{PI, TAU};

pub type Hours = f64;
pub type Degrees = f64;
pub type Radians = f64;

pub fn deg_to_rad(degrees: Degrees) -> Radians {
    PI * degrees / 180.
}

pub fn rad_to_deg(rad: Radians) -> Degrees {
    180. * rad / PI
}

pub fn hours_to_rad(hours: Hours) -> Radians {
    PI * hours / 12.
}

pub fn rad_to_hours(rad: Radians) -> Hours {
    12. * rad / PI
}

pub fn deg_to_hours(deg: Degrees) -> Hours {
    deg / 15.
}

pub fn hours_to_deg(hours: Hours) -> Degrees {
    hours * 15.
}

// Convert hms to hours or dms to degrees
pub fn ms_to_dec(d: u32, minutes: u32, seconds: f64) -> f64 {
    (d as f64) + (minutes as f64) / 60. + seconds / 3600.
}

pub fn dec_to_ms(dec: f64) -> (u32, u32, f64) {
    if dec < 0. {
        panic!("dec must not be negative");
    }

    let h = dec as u32;
    let m_raw = (dec - h as f64) * 60.;
    let m = m_raw as u32;
    let s = (m_raw - m as f64) * 60.;

    (h, m, s)
}

/// Calculates the Julian Date of a time
/// see https://scienceworld.wolfram.com/astronomy/JulianDate.html
fn calc_jd(time: chrono::DateTime<chrono::Utc>) -> Hours {
    let y = time.year() as f64;
    let m = time.month() as f64;
    let d = time.day() as f64;

    let mut jd = 367. * y;
    jd -= f64::floor(7. * (y + f64::floor((m + 9.) / 12.)) / 4.);
    jd -= f64::floor(3. * (f64::floor((y + (m - 9.) / 7.) / 100.) + 1.) / 4.);
    jd += f64::floor(275. * m / 9.);
    jd += d;
    jd += 1721028.5;
    jd + ms_to_dec(time.hour(), time.minute(), time.second() as f64) / 24.
}

// see https://thecynster.home.blog/2019/11/04/calculating-sidereal-time/
pub fn calculate_greenwich_sidereal_time(time: chrono::DateTime<chrono::Utc>) -> Hours {
    // The result will be off by the number of leap seconds different from this on the date given
    // TODO use the total number of leap seconds at the time given
    const LEAP_SECOND_TOTAL: u32 = 27;

    let jd_utc = calc_jd(time);

    let du = jd_utc - 2451545.0;
    let theta = rad_to_hours(modulo(
        TAU * (0.7790572732640 + 1.00273781191135448 * du),
        TAU,
    ));

    let poly = poly![
        0.014506,
        4612.156534,
        1.3915817,
        -0.00000044,
        -0.000029956,
        -0.0000000368
    ];
    let jd_tt = jd_utc + ((LEAP_SECOND_TOTAL as f64 + 32.184) / 3600.) / 24.; // Hours
    let t = (jd_tt - 2451545.0) / 36525.; // years

    let gmstp = deg_to_hours(modulo(poly.eval(t).unwrap() / 3600., 360.));

    modulo(theta + gmstp, 24.)
}

/// longitude in degrees
/// returns hours
pub fn calculate_local_sidereal_time(
    time: chrono::DateTime<chrono::Utc>,
    longitude: Degrees,
) -> Hours {
    let greenwich_sidereal_time = calculate_greenwich_sidereal_time(time);
    modulo(greenwich_sidereal_time + deg_to_hours(longitude), 24.)
}

/// longitude in degrees, ra in hours
/// returns hours
pub fn calculate_hour_angle(
    time: chrono::DateTime<chrono::Utc>,
    longitude: Degrees,
    ra: Hours,
) -> Hours {
    calculate_local_sidereal_time(time, longitude) - ra
}

pub fn calculate_alt_from_ha(ha: Hours, dec: Degrees, lat: Degrees) -> Degrees {
    let dec = deg_to_rad(dec);
    let lat = deg_to_rad(lat);
    rad_to_deg((dec.sin() * lat.sin() + dec.cos() * lat.cos() * ha.cos()).asin())
}

pub fn calculate_az_from_ha(ha: Hours, dec: Degrees, lat: Degrees) -> Degrees {
    let dec = deg_to_rad(dec);
    let lat = deg_to_rad(lat);

    let alt = calculate_alt_from_ha(ha, dec, lat);

    let a = rad_to_deg(((dec.sin() - alt.sin() * lat.sin()) / (alt.cos() * lat.cos())).acos())
        as Degrees;

    if 0. < ha.sin() {
        360. - a
    } else {
        a
    }
}

pub fn modulo(val: f64, base: f64) -> f64 {
    ((val % base) + base) % base
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_deg_to_rad() {
        assert_eq!(deg_to_rad(0.), 0.);
        assert_float_relative_eq!(deg_to_rad(55.), 0.9599311);
        assert_float_relative_eq!(deg_to_rad(-10.), -0.1745329);
    }

    #[test]
    fn test_rad_to_deg() {
        assert_eq!(rad_to_deg(0.), 0.);
        assert_float_relative_eq!(rad_to_deg(1.), 57.29578);
        assert_float_relative_eq!(rad_to_deg(-8.), -458.3662);
    }

    #[test]
    fn test_hours_to_rad() {
        assert_eq!(hours_to_rad(0.), 0.);
        assert_float_relative_eq!(hours_to_rad(1.), 0.261799, 1E-4);
        assert_float_relative_eq!(hours_to_rad(-8.), -2.0944, 1E-4);
    }

    #[test]
    fn test_rad_to_hours() {
        assert_eq!(rad_to_hours(0.), 0.);
        assert_float_relative_eq!(rad_to_hours(1.), 3.8197, 1E-4);
        assert_float_relative_eq!(rad_to_hours(-8.), -30.5577, 1E-4);
    }

    #[test]
    fn test_deg_to_hours() {
        assert_eq!(deg_to_hours(0.), 0.);
        assert_float_relative_eq!(deg_to_hours(1.), 0.0666666666666667);
        assert_float_relative_eq!(deg_to_hours(-8.), -0.53333333333333333);
    }

    #[test]
    fn test_hours_to_deg() {
        assert_eq!(hours_to_deg(0.), 0.);
        assert_float_relative_eq!(hours_to_deg(1.), 15.);
        assert_float_relative_eq!(hours_to_deg(-8.), -120.);
    }

    #[test]
    fn test_ms_to_dec() {
        assert_eq!(ms_to_dec(0, 0, 0.), 0.);
        assert_float_relative_eq!(ms_to_dec(1, 1, 1.), 1.0169444);
        assert_float_relative_eq!(-ms_to_dec(8, 8, 8.8), -8.1357778);
    }

    #[test]
    fn test_dec_to_ms() {
        assert_eq!(dec_to_ms(0.), (0, 0, 0.));
        let mut res;

        res = dec_to_ms(1.111);
        assert_eq!((res.0, res.1), (1, 6));
        assert_float_relative_eq!(res.2, 39.6);

        res = dec_to_ms(368.888);
        assert_eq!((res.0, res.1), (368, 53));
        assert_float_relative_eq!(res.2, 16.8)
    }

    #[test]
    fn test_calculate_greenwich_sidereal_time() {
        assert_float_relative_eq!(
            calculate_greenwich_sidereal_time(Utc.ymd(1969, 1, 6).and_hms(1, 5, 0)),
            8.1127421203,
            1E-4
        );
        assert_float_relative_eq!(
            calculate_greenwich_sidereal_time(Utc.ymd(2021, 1, 30).and_hms(21, 20, 0)),
            6.0219108930,
            1E-4
        );
    }

    #[test]
    fn test_calculate_local_sidereal_time() {
        assert_float_relative_eq!(
            calculate_local_sidereal_time(Utc.ymd(1969, 1, 6).and_hms(1, 5, 0), -55.5),
            4.4127385800,
            1E-4
        );
        assert_float_relative_eq!(
            calculate_local_sidereal_time(Utc.ymd(2021, 1, 30).and_hms(21, 20, 0), 90.),
            12.0219108930,
            1E-4
        );
    }

    #[test]
    fn test_calculate_hour_angle() {
        assert_float_relative_eq!(
            calculate_hour_angle(Utc.ymd(1969, 1, 6).and_hms(1, 5, 0), -55.5, -4.4),
            8.8127385800,
            1E-4
        );
        assert_float_relative_eq!(
            calculate_hour_angle(Utc.ymd(2021, 1, 30).and_hms(21, 20, 0), 90., 12.),
            0.0219108930,
            1E-4
        );
    }

    #[test]
    fn test_modulo() {
        assert_eq!(modulo(std::f64::consts::TAU, std::f64::consts::PI), 0.);
        assert_eq!(modulo(-365., 360.), 355.);
    }
}
