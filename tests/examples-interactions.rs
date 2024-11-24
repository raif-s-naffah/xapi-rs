// SPDX-License-Identifier: GPL-3.0-or-later

mod utils;

use std::str::FromStr;
use utils::read_to_string;
use xapi_rs::{ActivityDefinition, InteractionType, MyError, MyLanguageTag};

#[test]
fn test_true_false() -> Result<(), MyError> {
    let json = read_to_string("definition-true-false", true);

    let de_result = serde_json::from_str::<ActivityDefinition>(&json);
    assert!(de_result.is_ok());
    let ad = de_result.unwrap();

    assert!(ad.description(&MyLanguageTag::from_str("en")?).is_none());
    assert_eq!(
        ad.description(&MyLanguageTag::from_str("en-US")?).unwrap(),
        "Does the xAPI include the concept of statements?"
    );
    assert_eq!(
        ad.type_().unwrap(),
        "http://adlnet.gov/expapi/activities/cmi.interaction"
    );
    assert_eq!(ad.interaction_type().unwrap(), &InteractionType::TrueFalse);
    assert!(ad.correct_responses_pattern().is_some());
    let vec = ad.correct_responses_pattern().unwrap();
    assert_eq!(vec.len(), 1);
    assert!(vec.contains(&String::from("true")));

    Ok(())
}

#[test]
fn test_choice() {
    let json = read_to_string("definition-choice", true);

    let de_result = serde_json::from_str::<ActivityDefinition>(&json);
    assert!(de_result.is_ok());
    let ad = de_result.unwrap();

    assert_eq!(ad.interaction_type().unwrap(), &InteractionType::Choice);
    assert!(ad.correct_responses_pattern().is_some());
    let vec = ad.correct_responses_pattern().unwrap();
    assert_eq!(vec.len(), 1);
    assert!(vec.contains(&String::from("golf[,]tetris")));
    assert!(ad.choices().is_some());
    let choices = ad.choices().unwrap();
    assert_eq!(choices.len(), 4);
    assert_eq!(choices[0].id(), "golf");
    assert_eq!(choices[1].id(), "facebook");
    assert_eq!(choices[2].id(), "tetris");
    assert_eq!(choices[3].id(), "scrabble");
}

#[test]
fn test_fill_in() {
    let json = read_to_string("definition-fill-in", true);

    let de_result = serde_json::from_str::<ActivityDefinition>(&json);
    assert!(de_result.is_ok());
    let ad = de_result.unwrap();

    assert_eq!(ad.interaction_type().unwrap(), &InteractionType::FillIn);
    assert!(ad.correct_responses_pattern().is_some());
    let vec = ad.correct_responses_pattern().unwrap();
    assert_eq!(vec.len(), 1);
    assert!(vec.contains(&String::from("Bob's your uncle")));
}

#[test]
fn test_long_fill_in() {
    let json = read_to_string("definition-long-fill-in", true);

    let de_result = serde_json::from_str::<ActivityDefinition>(&json);
    assert!(de_result.is_ok());
    let ad = de_result.unwrap();

    assert_eq!(ad.interaction_type().unwrap(), &InteractionType::LongFillIn);
    assert!(ad.correct_responses_pattern().is_some());
    let vec = ad.correct_responses_pattern().unwrap();
    assert_eq!(vec.len(), 1);
    assert!(vec.contains(&String::from(
        "{case_matters=false}{lang=en}To store and provide access to learning experiences."
    )));
}

#[test]
fn test_matching() {
    const SOURCE_IDS: [&str; 4] = ["ben", "chris", "troy", "freddie"];

    let json = read_to_string("definition-matching", true);

    let de_result = serde_json::from_str::<ActivityDefinition>(&json);
    assert!(de_result.is_ok());
    let ad = de_result.unwrap();

    assert_eq!(ad.interaction_type().unwrap(), &InteractionType::Matching);
    assert!(ad.correct_responses_pattern().is_some());
    let vec = ad.correct_responses_pattern().unwrap();
    assert_eq!(vec.len(), 1);
    assert!(vec.contains(&String::from(
        "ben[.]3[,]chris[.]2[,]troy[.]4[,]freddie[.]1"
    )));
    assert!(ad.source().is_some());
    let source = ad.source().unwrap();
    assert_eq!(source.len(), 4);
    for i in 0..4 {
        assert_eq!(source[i].id(), SOURCE_IDS[i]);
    }
    assert!(ad.target().is_some());
    let target = ad.target().unwrap();
    assert_eq!(target.len(), 4);
    for i in 0..4 {
        assert_eq!(target[i].id(), &format!("{}", i + 1));
    }
}

#[test]
fn test_performance() {
    const STEP_IDS: [&str; 3] = ["pong", "dg", "lunch"];

    let json = read_to_string("definition-performance", true);

    let de_result = serde_json::from_str::<ActivityDefinition>(&json);
    assert!(de_result.is_ok());
    let ad = de_result.unwrap();

    assert_eq!(
        ad.interaction_type().unwrap(),
        &InteractionType::Performance
    );
    assert!(ad.correct_responses_pattern().is_some());
    let vec = ad.correct_responses_pattern().unwrap();
    assert_eq!(vec.len(), 1);
    assert!(vec.contains(&String::from("pong[.]1:[,]dg[.]:10[,]lunch[.]")));
    assert!(ad.steps().is_some());
    let steps = ad.steps().unwrap();
    assert_eq!(steps.len(), 3);
    for i in 0..3 {
        assert_eq!(steps[i].id(), STEP_IDS[i]);
    }
}

#[test]
fn test_sequencing() {
    const CHOICE_IDS: [&str; 4] = ["tim", "ben", "ells", "mike"];

    let json = read_to_string("definition-sequencing", true);

    let de_result = serde_json::from_str::<ActivityDefinition>(&json);
    assert!(de_result.is_ok());
    let ad = de_result.unwrap();

    assert_eq!(ad.interaction_type().unwrap(), &InteractionType::Sequencing);
    assert!(ad.correct_responses_pattern().is_some());
    let vec = ad.correct_responses_pattern().unwrap();
    assert_eq!(vec.len(), 1);
    assert!(vec.contains(&String::from("tim[,]mike[,]ells[,]ben")));
    assert!(ad.choices().is_some());
    let choices = ad.choices().unwrap();
    assert_eq!(choices.len(), 4);
    for i in 0..4 {
        assert_eq!(choices[i].id(), CHOICE_IDS[i]);
    }
}

#[test]
fn test_likert() {
    let json = read_to_string("definition-likert", true);

    let de_result = serde_json::from_str::<ActivityDefinition>(&json);
    assert!(de_result.is_ok());
    let ad = de_result.unwrap();

    assert_eq!(ad.interaction_type().unwrap(), &InteractionType::Likert);
    assert!(ad.correct_responses_pattern().is_some());
    let vec = ad.correct_responses_pattern().unwrap();
    assert_eq!(vec.len(), 1);
    assert!(vec.contains(&String::from("likert_3")));
    assert!(ad.scale().is_some());
    let scale = ad.scale().unwrap();
    assert_eq!(scale.len(), 4);
    for i in 0..4 {
        assert_eq!(scale[i].id(), &format!("likert_{}", i));
    }
}

#[test]
fn test_numeric() {
    let json = read_to_string("definition-numeric", true);

    let de_result = serde_json::from_str::<ActivityDefinition>(&json);
    assert!(de_result.is_ok());
    let ad = de_result.unwrap();

    assert_eq!(ad.interaction_type().unwrap(), &InteractionType::Numeric);
    assert!(ad.correct_responses_pattern().is_some());
    let vec = ad.correct_responses_pattern().unwrap();
    assert_eq!(vec.len(), 1);
    assert!(vec.contains(&String::from("4[:]")));
}

#[test]
fn test_other() {
    let json = read_to_string("definition-other", true);

    let de_result = serde_json::from_str::<ActivityDefinition>(&json);
    assert!(de_result.is_ok());
    let ad = de_result.unwrap();

    assert_eq!(ad.interaction_type().unwrap(), &InteractionType::Other);
    assert!(ad.correct_responses_pattern().is_some());
    let vec = ad.correct_responses_pattern().unwrap();
    assert_eq!(vec.len(), 1);
    assert!(vec.contains(&String::from("(35.937432,-86.868896)")));
}
