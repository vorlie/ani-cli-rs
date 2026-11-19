pub mod anikoto;
pub mod anikoto_cz;
mod common;

pub use anikoto::{
    AnikotoClient, AnikotoClientBuilder, provider_from_show_id,
    requires_hls_relay as anikoto_requires_hls_relay,
};
pub use anikoto_cz::{
    AnikotoCzClient, AnikotoCzClientBuilder, requires_hls_relay as anikoto_cz_requires_hls_relay,
    total_episodes_from_id as anikoto2_total_episodes,
};
