// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use tokio_rusqlite::rusqlite::Connection;

use super::document::DocumentError;

pub trait Query: Send + 'static {
    type Output: Send + 'static;

    fn run(self, conn: &Connection) -> Result<Self::Output, DocumentError>;
}
