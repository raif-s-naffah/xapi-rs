// SPDX-License-Identifier: GPL-3.0-or-later

use crate::data::{
    Attachment, DataError, Statement, StatementId, StatementResult, StatementResultId,
    ValidationError,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::error;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum StatementType {
    S(Box<Statement>),
    SId(Box<StatementId>),
    SR(StatementResult),
    SRId(StatementResultId),
}

impl StatementType {
    pub fn set_more(&mut self, val: &str) -> Result<(), DataError> {
        match self {
            StatementType::SR(x) => x.set_more(val),
            StatementType::SRId(x) => x.set_more(val),
            _ => {
                let msg = "Not a Statement variant";
                error!("{}", msg);
                Err(DataError::Validation(ValidationError::ConstraintViolation(
                    msg.into(),
                )))
            }
        }
    }

    /// Return TRUE if this inner instance is a collection and is empty. Return
    /// FALSE otherwise.
    pub fn is_empty(&self) -> bool {
        match self {
            StatementType::SR(x) => x.is_empty(),
            StatementType::SRId(x) => x.is_empty(),
            _ => false,
        }
    }

    /// If this inner instance is a single type, then return its `stored` timestamp
    /// if set or the Unix Epoch (1970-01-01 00:00:00 UTC) if it isn't.
    ///
    /// Alternatively, if this inner instance is a collection, then return the
    /// most recent `stored` timestamp of the collection's items. If `stored`
    /// was not set for all the items of the collection, then return the Unix
    /// Epoch.
    pub fn stored(&self) -> DateTime<Utc> {
        match self {
            StatementType::S(x) => x.stored().map_or(DateTime::UNIX_EPOCH, |x| *x),
            StatementType::SId(x) => x.stored().map_or(DateTime::UNIX_EPOCH, |x| *x),
            StatementType::SR(x) => {
                let mut stored = DateTime::UNIX_EPOCH;
                for s in x.statements() {
                    let ts = s.stored().map_or(&DateTime::UNIX_EPOCH, |x| x);
                    if ts > &stored {
                        stored = *ts
                    };
                }
                stored
            }
            StatementType::SRId(x) => {
                let mut stored = DateTime::UNIX_EPOCH;
                for s in x.statements() {
                    let ts = s.stored().map_or(&DateTime::UNIX_EPOCH, |x| x);
                    if ts > &stored {
                        stored = *ts
                    };
                }
                stored
            }
        }
    }

    /// Return the potentially empty collection of [Attachment]s.
    pub fn attachments(&self) -> Vec<Attachment> {
        match self {
            StatementType::S(x) => x.attachments().to_vec(),
            StatementType::SId(x) => x.attachments().to_vec(),
            StatementType::SR(x) => {
                let mut v = vec![];
                for s in x.statements() {
                    v.extend_from_slice(s.attachments());
                }
                v
            }
            StatementType::SRId(x) => {
                let mut v = vec![];
                for s in x.statements() {
                    v.extend_from_slice(s.attachments());
                }
                v
            }
        }
    }
}
