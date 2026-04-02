use hidpp_core::features::dpi::DpiList;
use hidpp_core::features::OnboardProfile;

#[derive(Clone)]
pub struct DeviceSnapshot {
    pub current_dpi: u16,
    pub dpi_list: DpiList,
    pub current_polling_hz: u16,
    pub supported_polling_hz: Vec<u16>,
    pub active_profile: u8,
    /// One entry per profile slot; None means unreadable.
    pub profiles: Vec<Option<OnboardProfile>>,
}
