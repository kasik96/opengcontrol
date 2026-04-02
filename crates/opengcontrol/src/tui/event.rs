use super::device_state::DeviceSnapshot;

pub enum AppEvent {
    DeviceData(Result<DeviceSnapshot, String>),
    WriteResult(Result<(), String>),
}
