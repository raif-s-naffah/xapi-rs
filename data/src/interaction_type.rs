// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt;
use serde::{Deserialize, Serialize};

/// Enumeration used in [ActivityDefinition][1]s.
///
/// Based on the variant used, the data formatting and purpose of other fields
/// in an [ActivityDefinition][1] instance are implied.
///
/// [1]: crate::ActivityDefinition
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub enum InteractionType {
    /// An _interaction_ with two possible responses: `true` or `false`.
    ///
    /// Format: Either `true` or `false`.
    #[default]
    #[serde(rename = "true-false")]
    TrueFalse,

    /// An _interaction_ with a number of possible choices from which the
    /// learner can select. This includes interactions in which the learner can
    /// select only one answer from the list and those where the learner can
    /// select multiple items.
    ///
    /// Format: A list of item `id`s delimited by \[,\]. If the response contains
    /// only one item, the delimiter shall not be used.
    #[serde(rename = "choice")]
    Choice,

    /// An _interaction_ which requires the learner to supply a short response
    /// in the form of one or more strings of characters. Typically, the correct
    /// response consists of part of a word, one word or a few words. "Short"
    /// means that the correct responses pattern and learner response strings
    /// are normally 250 characters or less.
    ///
    /// Format: A list of responses delimited by \[,\]. If the response contains
    /// only one item, the delimiter shall not be used.
    #[serde(rename = "fill-in")]
    FillIn,

    /// An _interaction_ which requires the learner to supply a response in the
    /// form of a long string of characters. "Long" means that the correct
    /// responses pattern and learner response strings are normally more than
    /// 250 characters.
    ///
    /// Format: A list of responses delimited by \[,\]. If the response contains
    /// only one item, the delimiter shall not be used.
    #[serde(rename = "long-fill-in")]
    LongFillIn,

    /// An _interaction_ where the learner is asked to match items in one set
    /// (the _source_ set) to items in another set (the _target_ set). Items do
    /// not have to pair off exactly and it is possible for multiple or zero
    /// _source_ items to be matched to a given _target_ and vice versa.
    ///
    /// Format: A list of matching pairs, where each pair consists of a _source_
    /// item `id` followed by a _target_ item `id`. Items can appear in multiple
    /// (or zero) pairs. Items within a pair are delimited by \[.\]. Pairs are
    /// delimited by \[,\].
    #[serde(rename = "matching")]
    Matching,

    /// An _interaction_ that requires the learner to perform a task that
    /// requires multiple steps.
    ///
    /// Format: A list of steps containing a step `id`s and the response to that
    /// step. Step ids are separated from responses by \[.\]. Steps are
    /// delimited by \[,\]. The response can be a String as in a fill-in
    /// interaction or a number range as in a numeric interaction.
    #[serde(rename = "performance")]
    Performance,

    /// An _interaction_ where the learner is asked to order items in a set.
    ///
    /// Format: An ordered list of item `id`s delimited by \[,\].
    #[serde(rename = "sequencing")]
    Sequencing,

    /// An _interaction_ which asks the learner to select from a discrete set
    /// of choices on a scale.
    ///
    /// Format: A single item `id`.
    #[serde(rename = "likert")]
    Likert,

    /// Any _interaction_ which requires a numeric response from the learner.
    ///
    /// Format: A range of numbers represented by a minimum and a maximum
    /// delimited by \[:\]. Where the range does not have a maximum or does
    /// not have a minimum, that number is omitted but the delimiter is
    /// still used. E.g. \[:\]4 indicates a maximum for 4 and no minimum.
    /// Where the correct response or learner's response is a single number
    /// rather than a range, the single number with no delimiter may be used.
    #[serde(rename = "numeric")]
    Numeric,

    /// Another type of _interaction_ that does not fit into the other variants.
    ///
    /// Format: Any format is valid within this string as appropriate for the
    /// type of interaction.
    #[serde(rename = "other")]
    Other,
}

impl fmt::Display for InteractionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InteractionType::TrueFalse => write!(f, "true-false"),
            InteractionType::Choice => write!(f, "choice"),
            InteractionType::FillIn => write!(f, "fill-in"),
            InteractionType::LongFillIn => write!(f, "long-fill-in"),
            InteractionType::Matching => write!(f, "matching"),
            InteractionType::Performance => write!(f, "performance"),
            InteractionType::Sequencing => write!(f, "sequencing"),
            InteractionType::Likert => write!(f, "likert"),
            InteractionType::Numeric => write!(f, "numeric"),
            InteractionType::Other => write!(f, "other"),
        }
    }
}
