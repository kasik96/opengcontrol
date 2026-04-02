use std::sync::mpsc;
use std::time::Duration;

use hidpp_core::features::{AdjustableDpi, ButtonAction, OnboardProfiles, PollingRate};
use hidpp_core::HidppError;

use crate::context::DeviceContext;

use super::device_state::DeviceSnapshot;
use super::event::AppEvent;

pub enum PollerCommand {
    Refresh,
    SetDpi(u16),
    SetPolling(u16),
    SwitchProfile(u8),
    SetButtonAction { profile_idx: u8, button_idx: u8, action: ButtonAction },
    Quit,
}

const AUTO_REFRESH: Duration = Duration::from_secs(3);

pub fn spawn_poller(
    ctx: DeviceContext,
    tx: mpsc::SyncSender<AppEvent>,
) -> mpsc::SyncSender<PollerCommand> {
    let (cmd_tx, cmd_rx) = mpsc::sync_channel::<PollerCommand>(8);

    std::thread::spawn(move || {
        // Initial snapshot
        tx.send(AppEvent::DeviceData(read_snapshot(&ctx))).ok();

        loop {
            match cmd_rx.recv_timeout(AUTO_REFRESH) {
                Ok(PollerCommand::Refresh) => {
                    tx.send(AppEvent::DeviceData(read_snapshot(&ctx))).ok();
                }
                Ok(PollerCommand::SetDpi(value)) => {
                    let result = AdjustableDpi::set_dpi(ctx.device(), 0, value)
                        .map_err(|e| e.to_string());
                    if result.is_ok() {
                        // Best-effort: persist to the active profile in flash.
                        // Ignore errors here so the live change is still confirmed.
                        let _ = persist_dpi(&ctx, value);
                    }
                    tx.send(AppEvent::WriteResult(result)).ok();
                    tx.send(AppEvent::DeviceData(read_snapshot(&ctx))).ok();
                }
                Ok(PollerCommand::SetPolling(hz)) => {
                    let supported = ctx.device_info().supported_polling_rates();
                    let result = PollingRate::set_rate_hz(ctx.device(), hz, supported)
                        .map_err(|e| e.to_string());
                    tx.send(AppEvent::WriteResult(result)).ok();
                    tx.send(AppEvent::DeviceData(read_snapshot(&ctx))).ok();
                }
                Ok(PollerCommand::SwitchProfile(idx)) => {
                    let result = OnboardProfiles::set_active_profile(ctx.device(), idx)
                        .map_err(|e| e.to_string());
                    tx.send(AppEvent::WriteResult(result)).ok();
                    tx.send(AppEvent::DeviceData(read_snapshot(&ctx))).ok();
                }
                Ok(PollerCommand::SetButtonAction { profile_idx, button_idx, action }) => {
                    let result = set_button_action(&ctx, profile_idx, button_idx, action);
                    tx.send(AppEvent::WriteResult(result)).ok();
                    tx.send(AppEvent::DeviceData(read_snapshot(&ctx))).ok();
                }
                Ok(PollerCommand::Quit) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    tx.send(AppEvent::DeviceData(read_snapshot(&ctx))).ok();
                }
            }
        }
    });

    cmd_tx
}

fn set_button_action(
    ctx: &DeviceContext,
    profile_idx: u8,
    button_idx: u8,
    action: ButtonAction,
) -> Result<(), String> {
    let mut profile = OnboardProfiles::read_profile(ctx.device(), profile_idx)
        .map_err(|e| e.to_string())?;
    if let Some(a) = profile.button_assignments.iter_mut().find(|a| a.button_index == button_idx) {
        a.action = action;
    }
    OnboardProfiles::write_profile(ctx.device(), &profile)
        .map_err(|e| e.to_string())
}

/// Update the active DPI slot in the active onboard profile and write it to flash.
fn persist_dpi(ctx: &DeviceContext, dpi: u16) -> Result<(), HidppError> {
    let active = OnboardProfiles::get_active_profile(ctx.device())?;
    let mut profile = OnboardProfiles::read_profile(ctx.device(), active)?;
    let slot = profile.active_dpi_slot as usize;
    if slot < profile.dpi_slots.len() {
        profile.dpi_slots[slot] = dpi;
    } else if !profile.dpi_slots.is_empty() {
        profile.dpi_slots[0] = dpi;
    } else {
        profile.dpi_slots.push(dpi);
    }
    OnboardProfiles::write_profile(ctx.device(), &profile)
}

fn read_snapshot(ctx: &DeviceContext) -> Result<DeviceSnapshot, String> {
    let info = ctx.device_info();

    let current_dpi = AdjustableDpi::get_dpi(ctx.device(), 0)
        .map_err(|e| e.to_string())?;

    let dpi_list = AdjustableDpi::get_dpi_list(ctx.device(), 0)
        .map_err(|e| e.to_string())?;

    let current_polling_hz = PollingRate::get_rate_hz(ctx.device())
        .map_err(|e| e.to_string())?;

    let supported_polling_hz = PollingRate::get_supported_rates(ctx.device())
        .unwrap_or_else(|_| info.supported_polling_rates().to_vec());

    let active_profile = OnboardProfiles::get_active_profile(ctx.device())
        .unwrap_or(0);

    // The device sometimes returns 0 for profile_count (e.g. G305 SE).
    // Fall back to the count declared in the device registry.
    let profile_count = {
        let from_device = OnboardProfiles::get_info(ctx.device())
            .map(|i| i.profile_count)
            .unwrap_or(0);
        if from_device > 0 {
            from_device
        } else {
            ctx.device_info().profile_count().max(1)
        }
    };

    let mut profiles = Vec::with_capacity(profile_count as usize);
    for i in 0..profile_count {
        let p = OnboardProfiles::read_profile(ctx.device(), i).ok();
        profiles.push(p);
    }

    Ok(DeviceSnapshot {
        current_dpi,
        dpi_list,
        current_polling_hz,
        supported_polling_hz,
        active_profile,
        profiles,
    })
}
