use crate::request::*;
use crate::util::*;
use crate::{response, StarAdventurer};
use crate::{try_connected, AlpacaState};
use proc_macros::alpaca_handler;
use rocket::State;

/* Action */
#[alpaca_handler]
pub async fn put_action(data: ActionData, state: &AlpacaState) -> AscomResult<String> {
    match &*data.action {
        "complete_dec_slew" => {
            try_connected!(state, sa, {
                sa.complete_dec_slew().await?;
                Ok("".to_string())
            })
        }
        _ => Err(AscomError::from_msg(
            AscomErrorType::ActionNotImplemented,
            "Action not implemented".to_string(),
        )),
    }
}

/* Command */
#[alpaca_handler]
pub async fn put_command_blind(_data: CommandData, _state: &AlpacaState) -> AscomResult<String> {
    Err(AscomError::from_msg(
        AscomErrorType::ActionNotImplemented,
        "Blind commands not accepted".to_string(),
    ))
}

#[alpaca_handler]
pub async fn put_command_bool(_data: CommandData, _state: &AlpacaState) -> AscomResult<bool> {
    Err(AscomError::from_msg(
        AscomErrorType::ActionNotImplemented,
        "Bool commands not accepted".to_string(),
    ))
}

#[alpaca_handler]
pub async fn put_command_string(_data: CommandData, _state: &AlpacaState) -> AscomResult<String> {
    Err(AscomError::from_msg(
        AscomErrorType::ActionNotImplemented,
        "String commands not accepted".to_string(),
    ))
}

/* Connected */
#[alpaca_handler]
pub async fn get_connected(state: &AlpacaState) -> AscomResult<bool> {
    Ok(state.sa.read().await.is_some())
}

#[alpaca_handler]
pub async fn put_connected(data: SetConnectedData, state: &AlpacaState) -> AscomResult<()> {
    let mut sa = state.sa.write().await;
    match (&*sa, data.connected) {
        (Some(_), false) => {
            *sa = {
                log::warn!("Disconnecting");
                None
            }
        }
        (None, true) => {
            let v = StarAdventurer::new(&state.config).await;
            if let Err(e) = v {
                log::error!("Couldn't connect to StarAdventurer: {}", &e);
                return Err(e);
            } else {
                log::info!("Connected")
            }
            *sa = Some(v.unwrap())
        }
        _ => (),
    };

    Ok(())
}

#[alpaca_handler]
pub async fn get_description(_state: &AlpacaState) -> AscomResult<&'static str> {
    Ok("StarAdventurer")
}

#[alpaca_handler]
pub async fn get_driver_info(_state: &AlpacaState) -> AscomResult<&'static str> {
    Ok("Rust ALPACA driver for Star Adventurer")
}

#[alpaca_handler]
pub async fn get_driver_version(_state: &AlpacaState) -> AscomResult<&'static str> {
    Ok(env!("CARGO_PKG_VERSION"))
}

#[alpaca_handler]
pub async fn get_interface_version(_state: &AlpacaState) -> AscomResult<i32> {
    Ok(3)
}

#[alpaca_handler]
pub async fn get_name(_state: &AlpacaState) -> AscomResult<&'static str> {
    Ok("StarAdventurer")
}

#[alpaca_handler]
pub async fn get_supported_actions(_state: &AlpacaState) -> AscomResult<&[&str]> {
    Ok(&[])
}
