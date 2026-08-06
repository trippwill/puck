// SPDX-License-Identifier: MPL-2.0

use cosmic::cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic::cosmic_config::{self, CosmicConfigEntry};

#[derive(Debug, Default, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    demo: String,
}
