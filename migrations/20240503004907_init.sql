-- Add migration script here

DROP TABLE IF EXISTS member;
DROP TABLE IF EXISTS actor_ifi;
DROP TABLE IF EXISTS actor;
DROP TABLE IF EXISTS ifi;
DROP TABLE IF EXISTS verb;
DROP TABLE IF EXISTS activity;
DROP TABLE IF EXISTS state;
DROP TABLE IF EXISTS agent_profile;
DROP TABLE IF EXISTS activity_profile;
DROP TABLE IF EXISTS ctx_actors;
DROP TABLE IF EXISTS ctx_activities;
DROP TABLE IF EXISTS context;
DROP TABLE IF EXISTS result;
DROP TABLE IF EXISTS obj_statement;
DROP TABLE IF EXISTS obj_statement_ref;
DROP TABLE IF EXISTS obj_actor;
DROP TABLE IF EXISTS obj_activity;
DROP TABLE IF EXISTS statement;
DROP TABLE IF EXISTS attachments;
DROP TABLE IF EXISTS attachment;

-- Actor's `name` field values are stored and looked up as case-insensitive.
--
CREATE EXTENSION IF NOT EXISTS "citext";

-- Actor stuff
-- ------------

-- Stores the Inverse Functional Identifiers used in Agents and Identified
-- Groups.
-- The `kind` column is a numeric enumeration indicating how to interpret the
-- `value`. Possible values for `kind` are...
-- 0 -> email address (or `mbox` in xAPI parlance). Note i only store
--      the email address proper w/o the `mailto` scheme.
-- 1 -> hex-encoded SHA1 hash of a mailto IRI; i.e. 40-character string.
-- 2 -> OpenID URI identifying the owner.
-- 3 -> account on an existing system stored as a single string by catenating
--      the `home_page` URL, a ':' symbol followed by a `name` (the username
--      of the account holder).
--
CREATE TABLE IF NOT EXISTS ifi (
    id SERIAL NOT NULL PRIMARY KEY,
    kind SMALLINT NOT NULL DEFAULT 0,
    value TEXT NOT NULL,
    UNIQUE (kind, value)
);
INSERT INTO ifi (kind, value) VALUES (0, 'admin@my.xapi.net');  -- 1

-- Stores an Agent or Identified Group `name`, and fingerprint. Distinguishing
-- between an Agent and a Group is done w/ a boolean flag.
--
CREATE TABLE IF NOT EXISTS actor (
    id SERIAL NOT NULL PRIMARY KEY,
    fp BIGINT NOT NULL UNIQUE,
    name CITEXT,
    is_group BOOLEAN NOT NULL DEFAULT FALSE
);
INSERT INTO actor (name, fp, is_group) VALUES ('lars', 0, false);  -- 1

-- Stores associations between Actors and their IFIs.
--
CREATE TABLE IF NOT EXISTS actor_ifi (
    actor_id INTEGER NOT NULL,
    ifi_id INTEGER NOT NULL,
    PRIMARY KEY(actor_id, ifi_id),
    CONSTRAINT fk10 FOREIGN KEY(actor_id) REFERENCES actor(id),
    CONSTRAINT fk11 FOREIGN KEY(ifi_id) REFERENCES ifi(id)
);
INSERT INTO actor_ifi (actor_id, ifi_id) VALUES (1, 1);

-- Encodes the relation between Agents and Groups. It helps answer questions
-- like who are the Agents belonging to a Group, and is an Agent a member of
-- a Group, and if yes, which one.
--
CREATE TABLE IF NOT EXISTS member (
    group_id INTEGER NOT NULL,
    agent_id INTEGER NOT NULL,
    PRIMARY KEY(group_id, agent_id),
    CONSTRAINT fk12 FOREIGN KEY(group_id) REFERENCES actor(id),
    CONSTRAINT fk13 FOREIGN KEY(agent_id) REFERENCES actor(id)
);


-- Verb stuff
-- -----------
-- Stores Verb properties.
--
CREATE TABLE IF NOT EXISTS verb (
    id SERIAL NOT NULL PRIMARY KEY,
    iri TEXT NOT NULL UNIQUE,
    display JSONB
);

-- this one is treated differently in the xAPI...
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/voided', '{ "en": "voided"}');  -- 1

INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/answered', '{ "en": "answered"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/asked', '{ "en": "asked"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/attempted', '{ "en": "attempted"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/attended', '{ "en": "attended"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/commented', '{ "en": "commented"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/exited', '{ "en": "exited"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/experienced', '{ "en": "experienced"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/imported', '{ "en": "imported"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/interacted', '{ "en": "interacted"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/launched', '{ "en": "launched"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/mastered', '{ "en": "mastered"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/preferred', '{ "en": "preferred"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/progressed', '{ "en": "progressed"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/registered', '{ "en": "registered"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/shared', '{ "en": "shared"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/logged-in', '{ "en": "logged-in"}');
INSERT INTO verb (iri, display) VALUES ('http://adlnet.gov/expapi/verbs/logged-out', '{ "en": "logged-out"}');


-- Activity
-- ---------
-- Stores Activity properties.
--
CREATE TABLE IF NOT EXISTS activity (
    id SERIAL NOT NULL PRIMARY KEY,
    iri TEXT NOT NULL UNIQUE,
    definition JSONB
);

-- Activity State (or just State)
-- -------------------------------

-- Trigger to keep up-to-date timestamp columns named `updated`.
--
CREATE OR REPLACE FUNCTION update_timestamp() RETURNS TRIGGER AS $$
BEGIN
    new.updated := NOW();
    RETURN new;
END;
$$ LANGUAGE 'plpgsql';

CREATE TABLE IF NOT EXISTS state (
    activity_id INTEGER NOT NULL,
    agent_id INTEGER NOT NULL,
    registration UUID NOT NULL,
    state_id TEXT NOT NULL,
    document TEXT NOT NULL,
    updated TIMESTAMPTZ NULL DEFAULT NOW(),
    PRIMARY KEY(activity_id, agent_id, registration, state_id),
    CONSTRAINT fk20 FOREIGN KEY(activity_id) REFERENCES activity(id),
    CONSTRAINT fk21 FOREIGN KEY(agent_id) REFERENCES actor(id)
);

CREATE OR REPLACE TRIGGER state_updated
  BEFORE INSERT OR UPDATE ON state
  FOR EACH ROW EXECUTE FUNCTION update_timestamp();


-- Agent Profile
-- --------------
--
CREATE TABLE IF NOT EXISTS agent_profile (
    agent_id INTEGER NOT NULL,
    profile_id TEXT NOT NULL,
    document TEXT NOT NULL,
    updated TIMESTAMPTZ NULL DEFAULT NOW(),
    PRIMARY KEY(agent_id, profile_id),
    CONSTRAINT fk30 FOREIGN KEY(agent_id) REFERENCES actor(id)
);

CREATE OR REPLACE TRIGGER agent_profile_updated
  BEFORE INSERT OR UPDATE ON agent_profile
  FOR EACH ROW EXECUTE FUNCTION update_timestamp();


-- Activity Profile
-- -----------------
--
CREATE TABLE IF NOT EXISTS activity_profile (
    activity_id INTEGER NOT NULL,
    profile_id TEXT NOT NULL,
    document TEXT NOT NULL,
    updated TIMESTAMPTZ NULL DEFAULT NOW(),
    PRIMARY KEY(activity_id, profile_id),
    CONSTRAINT fk40 FOREIGN KEY(activity_id) REFERENCES activity(id)
);

CREATE OR REPLACE TRIGGER activity_profile_updated
  BEFORE INSERT OR UPDATE ON activity_profile
  FOR EACH ROW EXECUTE FUNCTION update_timestamp();


-- Statement stuff
-- ----------------

-- xAPI Result (which we call XResult) table.
--
CREATE TABLE IF NOT EXISTS result (
    id SERIAL NOT NULL PRIMARY KEY,
    score_scaled REAL,
    score_raw REAL,
    score_min REAL,
    score_max REAL,
    success BOOLEAN,
    completion BOOLEAN,
    response TEXT,
    duration TEXT,
    extensions JSONB
);

-- xAPI Context table.
--
CREATE TABLE IF NOT EXISTS context (
    id SERIAL NOT NULL PRIMARY KEY,
    registration UUID,
    instructor_id INTEGER,
    team_id INTEGER,
    revision TEXT,
    platform TEXT,
    language TEXT,
    statement UUID,
    extensions JSONB,

    CONSTRAINT fk50 FOREIGN KEY(instructor_id) REFERENCES actor(id),
    CONSTRAINT fk51 FOREIGN KEY(team_id) REFERENCES actor(id)
);

-- Context Activities properties.
-- The `kind` column encodes the 4 variants of `context_activities` a row
-- represents.
-- 0 -> parent,
-- 1 -> grouping,
-- 2 -> category, and
-- 3 -> other.
--
CREATE TABLE IF NOT EXISTS ctx_activities (
    context_id INTEGER NOT NULL,
    kind SMALLINT NOT NULL DEFAULT 0,
    activity_id INTEGER NOT NULL,
    PRIMARY KEY(context_id, kind, activity_id),

    CONSTRAINT fk52 FOREIGN KEY(context_id) REFERENCES context(id),
    CONSTRAINT fk53 FOREIGN KEY(activity_id) REFERENCES activity(id)
);

-- Use the same table for both `context_agents` and `context_groups`.
--
CREATE TABLE IF NOT EXISTS ctx_actors (
    id SERIAL NOT NULL PRIMARY KEY,
    context_id INTEGER NOT NULL,
    actor_id INTEGER NOT NULL,
    relevant_types JSONB,

    CONSTRAINT fk54 FOREIGN KEY(actor_id) REFERENCES actor(id)
);

-- The `fp` column is the _fingerprint_ value of the Statement. We use
-- fingerprints to assert _Equivalence_ of two Statements. BTW, when computing
-- the fingerprint value, neither the `id` nor the `uuid` columns are taken
-- into consideration. For all intents and purposes, the `fp` column acts as
-- the unique object identifier. However that field is not UNIQUE.  2 or more
-- records w/ different primary keys may refer to the same Statement (same fp)
-- but represent different 'states' of said Statement.
-- The `onject_kind` column encodes the Statement's Object alternatives:
--   0 -> activity.
--   1 -> agent,
--   2 -> group,
--   3 -> statement-ref, and
--   4 -> sub-statement.
-- Details of the statement's object are then stored-in/fetched-from the
-- corresponding `obj_xxx`. Those tables, except for the `obj_statement_ref`
-- use a compound primary key consisting of (a) the owner's statement row ID,
-- and (b) the corresponding object row ID in their own table. The exception
-- for `obj_statement_ref` substitutes a `statement.uuid` for the (b) part.
--
CREATE TABLE IF NOT EXISTS statement (
    id SERIAL NOT NULL PRIMARY KEY,
    fp BIGINT NOT NULL,
    uuid UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    voided BOOLEAN NOT NULL DEFAULT FALSE,
    actor_id INTEGER NOT NULL,
    verb_id INTEGER NOT NULL,
    object_kind SMALLINT NOT NULL DEFAULT 0, -- i.e. activity
    result_id INTEGER,
    context_id INTEGER,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    stored TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    authority_id INTEGER NULL,
    version TEXT NULL,
    exact JSONB,

    CONSTRAINT fk55 FOREIGN KEY(actor_id) REFERENCES actor(id),
    CONSTRAINT fk56 FOREIGN KEY(verb_id) REFERENCES verb(id),
    CONSTRAINT fk57 FOREIGN KEY(result_id) REFERENCES result(id),
    CONSTRAINT fk58 FOREIGN KEY(context_id) REFERENCES context(id),
    CONSTRAINT fk59 FOREIGN KEY(authority_id) REFERENCES actor(id)
);

-- Encodes the association between a Statement and an Activity when a
-- Statement's Object is an Activity.
--
CREATE TABLE IF NOT EXISTS obj_activity (
    statement_id INTEGER NOT NULL,
    activity_id INTEGER NOT NULL,
    PRIMARY KEY(statement_id, activity_id),

    CONSTRAINT fk60 FOREIGN KEY(statement_id) REFERENCES statement(id),
    CONSTRAINT fk61 FOREIGN KEY(activity_id) REFERENCES activity(id)
);

-- Encodes the association between a Statement and an Actor when a
-- Statement's Object is either an Agent or a Group.
--
CREATE TABLE IF NOT EXISTS obj_actor (
    statement_id INTEGER NOT NULL,
    actor_id INTEGER NOT NULL,
    PRIMARY KEY(statement_id, actor_id),

    CONSTRAINT fk70 FOREIGN KEY(statement_id) REFERENCES statement(id),
    CONSTRAINT fk71 FOREIGN KEY(actor_id) REFERENCES actor(id)
);

-- Encodes the association between a Statement and another Statement when a
-- Statement's Object is a StatementRef. The `uuid` column is that of the
-- referenced Statement Object.
--
CREATE TABLE IF NOT EXISTS obj_statement_ref (
    statement_id INTEGER NOT NULL,
    uuid UUID NOT NULL,
    PRIMARY KEY(statement_id, uuid),

    CONSTRAINT fk80 FOREIGN KEY(statement_id) REFERENCES statement(id)
);

-- Encodes the association between two Statement when a Statement's Object is
-- another Statement (represented by an instance of SubStatement). An instance
-- of SubStatement is a Statement but w/ less populated fields and a subset of
-- target Object types.
--
CREATE TABLE IF NOT EXISTS obj_statement (
    statement_id INTEGER NOT NULL,
    sub_statement_id INTEGER NOT NULL,
    PRIMARY KEY(statement_id, sub_statement_id),

    CONSTRAINT fk91 FOREIGN KEY(statement_id) REFERENCES statement(id),
    CONSTRAINT fk92 FOREIGN KEY(sub_statement_id) REFERENCES statement(id)
);

-- Attachments stuff
-- ------------------
--
CREATE TABLE IF NOT EXISTS attachment (
    id SERIAL NOT NULL PRIMARY KEY,
    usage_type TEXT NOT NULL,
    display JSONB,
    description JSONB,
    content_type TEXT NOT NULL,
    length BIGINT NOT NULL,
    sha2 TEXT NOT NULL,
    file_url TEXT
);

-- Encodes the association between a Statement or SubStatement and an Attachment.
--
CREATE TABLE IF NOT EXISTS attachments (
    statement_id INTEGER NOT NULL,
    attachment_id INTEGER NOT NULL,
    PRIMARY KEY(statement_id, attachment_id),

    CONSTRAINT fka1 FOREIGN KEY(statement_id) REFERENCES statement(id),
    CONSTRAINT fka2 FOREIGN KEY(attachment_id) REFERENCES attachment(id)
);
