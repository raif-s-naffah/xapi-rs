// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;
use sqlx::{
    postgres::types::PgCiText,
    types::{
        chrono::{DateTime, Utc},
        Json,
    },
    FromRow,
};
use uuid::Uuid;

use crate::{
    data::{ActivityDefinition, Extensions, LanguageMap},
    Statement,
};

// ===== actor stuff ==========================================================

/// Representation of an `ifi` row.
///
/// `kind` is a numeric enumeration indicating how to interpret the `value`.
/// Possible values are:
///
/// 0: email address (or `mbox` in xAPI parlance). Note i only store
///    the email address proper w/o the `mailto` scheme.
/// 1: hex-encoded SHA1 hash of a mailto IRI; i.e. 40-character string.
/// 2: OpenID URI identifying the owner.
/// 3: account on an existing system e.g. an LMS or intranet, stored
///    as a single string by catenating the `home_page` URL, a ':' symbol
///    followed by a `name` (the username of the account holder).
#[derive(Debug, FromRow)]
pub(crate) struct TIfi {
    pub(crate) id: i32,
    pub(crate) kind: i16,
    pub(crate) value: String,
}

impl fmt::Display for TIfi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{id: {}, kind: {}, value: '{}' }}",
            self.id, self.kind, self.value
        )
    }
}

/// Representation of an `actor` row.
#[derive(Debug, FromRow)]
pub(crate) struct TActor {
    pub(crate) id: i32,
    pub(crate) name: Option<PgCiText>,
    pub(crate) is_group: bool,
}

impl fmt::Display for TActor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut display = "".to_owned();
        if self.is_group {
            display.push_str("G_");
        } else {
            display.push_str("A_");
        }
        display.push_str(&self.id.to_string());
        // handle `name` if some...
        if self.name.is_some() {
            let n = self.name.clone().unwrap();
            display.push_str(format!("[{}]", n).as_str());
        }
        write!(f, "{}", display)
    }
}

#[derive(Debug, FromRow)]
pub(crate) struct TActorIfi {
    pub(crate) actor_id: i32,
    pub(crate) ifi_id: i32,
}

// ===== verb stuff ===========================================================

/// Representation of a `verb` row.
#[derive(Debug, FromRow)]
pub(crate) struct TVerb {
    pub(crate) id: i32,
    pub(crate) iri: String,
    pub(crate) display: Option<Json<LanguageMap>>,
}

// ===== state stuff ==========================================================

/// Representation of an `activity` row.
#[derive(Debug, FromRow)]
pub(crate) struct TActivity {
    pub(crate) id: i32,
    pub(crate) iri: String,
    pub(crate) definition: Option<Json<ActivityDefinition>>,
}

/// Representation of a `state` row.
#[derive(Debug, FromRow)]
pub(crate) struct TState {
    #[allow(dead_code)]
    activity_id: i32,
    #[allow(dead_code)]
    agent_id: i32,
    #[allow(dead_code)]
    registration: Uuid,
    pub(crate) state_id: String,
    pub(crate) document: String,
    pub(crate) updated: DateTime<Utc>,
}

// ===== actor_profile stuff ==================================================

/// Representation of an `agent_profile` row.
#[derive(Debug, FromRow)]
pub(crate) struct TAgentProfile {
    #[allow(dead_code)]
    agent_id: i32,
    pub(crate) profile_id: String,
    pub(crate) document: String,
    pub(crate) updated: DateTime<Utc>,
}

// ===== activity_profile stuff ===============================================

/// Representation of an `activity_profile` row.
#[derive(Debug, FromRow)]
pub(crate) struct TActivityProfile {
    #[allow(dead_code)]
    activity_id: i32,
    pub(crate) profile_id: String,
    pub(crate) document: String,
    pub(crate) updated: DateTime<Utc>,
}

// ===== statement stuff ======================================================

/// Representation of a `result` row.
#[derive(Debug, FromRow)]
pub(crate) struct TResult {
    #[allow(dead_code)]
    id: i32,
    pub(crate) score_scaled: Option<f32>,
    pub(crate) score_raw: Option<f32>,
    pub(crate) score_min: Option<f32>,
    pub(crate) score_max: Option<f32>,
    pub(crate) success: Option<bool>,
    pub(crate) completion: Option<bool>,
    pub(crate) response: Option<String>,
    pub(crate) duration: Option<String>,
    pub(crate) extensions: Option<Json<Extensions>>,
}

/// Representation of a `context` row.
#[derive(Debug, FromRow)]
pub(crate) struct TContext {
    pub(crate) id: i32,
    pub(crate) registration: Option<Uuid>,
    pub(crate) instructor_id: Option<i32>,
    pub(crate) team_id: Option<i32>,
    pub(crate) revision: Option<String>,
    pub(crate) platform: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) statement: Option<Uuid>,
    pub(crate) extensions: Option<Json<Extensions>>,
}

/// Representation of a `ctx_activities` row.
///
/// `kind` is a numeric enumeration indicating how to interpret the Activity
/// w/ the given row ID. Possible values are:
///
///   0: parent,
///   1: grouping,
///   2: category, and
///   3: other.
#[derive(Debug, FromRow)]
pub(crate) struct TCtxActivities {
    #[allow(dead_code)]
    context_id: i32,
    pub(crate) kind: i16,
    pub(crate) activity_id: i32,
}

/// Representation of a `ctx_actors` row. The xAPI specifications state that
/// a Statement Context may have both a `context_agents` as well as a
/// `context_groups`.
///
/// We store both in the same table. Whether it's a member of the Agents or the
/// Group collection is provided by the value of the Actor's boolean flag:
/// `is_group`.
#[derive(Debug, FromRow)]
pub(crate) struct TCtxActors {
    #[allow(dead_code)]
    context_id: i32,
    #[allow(dead_code)]
    kind: i16,
    pub(crate) actor_id: i32,
    pub(crate) relevant_types: Option<Json<Vec<String>>>,
}

/// Representation of a `statement` row. The `onject_kind` field encodes the
/// Statement's Object alternatives:
///
///   0 -> activity.
///   1 -> agent,
///   2 -> group,
///   3 -> statement-ref, and
///   4 -> sub-statement.
#[derive(Debug, FromRow)]
pub(crate) struct TStatement {
    pub(crate) id: i32,
    pub(crate) fp: i64,
    pub(crate) uuid: Uuid,
    pub(crate) voided: bool,
    pub(crate) actor_id: i32,
    pub(crate) verb_id: i32,
    pub(crate) object_kind: i16,
    pub(crate) result_id: Option<i32>,
    pub(crate) context_id: Option<i32>,
    pub(crate) timestamp: DateTime<Utc>,
    pub(crate) stored: DateTime<Utc>,
    pub(crate) authority_id: Option<i32>,
    pub(crate) version: Option<String>,
    pub(crate) exact: Option<Json<Statement>>,
}

/// Representation of a `obj_activity` row.
#[derive(Debug, FromRow)]
pub(crate) struct TObjActivity {
    #[allow(dead_code)]
    statement_id: i32,
    pub(crate) activity_id: i32,
}

/// Representation of a `obj_actor` row.
#[derive(Debug, FromRow)]
pub(crate) struct TObjActor {
    #[allow(dead_code)]
    statement_id: i32,
    pub(crate) actor_id: i32,
}

/// Representation of a `obj_statement_ref` row.
#[derive(Debug, FromRow)]
pub(crate) struct TObjStatementRef {
    #[allow(dead_code)]
    statement_id: i32,
    pub(crate) uuid: Uuid,
}

/// Representation of a `obj_statement` row.
#[derive(Debug, FromRow)]
pub(crate) struct TObjStatement {
    #[allow(dead_code)]
    statement_id: i32,
    pub(crate) sub_statement_id: i32,
}

// ===== attachments stuff ====================================================

/// Representation of a `obj_statement` row.
#[derive(Debug, FromRow)]
pub(crate) struct TAttachment {
    #[allow(dead_code)]
    pub(crate) id: i32,
    pub(crate) usage_type: String,
    pub(crate) display: Json<LanguageMap>,
    pub(crate) description: Option<Json<LanguageMap>>,
    pub(crate) content_type: String,
    pub(crate) length: i64,
    pub(crate) sha2: String,
    pub(crate) file_url: Option<String>,
}

/// Representation of a `attachments` row.
#[derive(Debug, FromRow)]
pub(crate) struct TAttachments {
    #[allow(dead_code)]
    statement_id: i32,
    pub(crate) attachment_id: i32,
}
