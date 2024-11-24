// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    data::{Attachment, EMPTY_LANGUAGE_MAP},
    db::{
        schema::{TAttachment, TAttachments},
        RowID,
    },
    emit_db_error, MyError,
};
use sqlx::PgPool;
use tracing::{debug, instrument};

impl TryFrom<TAttachment> for Attachment {
    type Error = MyError;

    fn try_from(value: TAttachment) -> Result<Self, Self::Error> {
        let mut builder = Attachment::builder()
            .usage_type(&value.usage_type)?
            .with_display(value.display.0)?;
        if let Some(map) = value.description {
            builder = builder.with_description(map.0)?;
        }
        builder = builder
            .content_type(&value.content_type)?
            .length(value.length)?
            .sha2(&value.sha2)?;
        let res = if let Some(url) = value.file_url {
            builder.file_url(url.as_str())?.build()?
        } else {
            builder.build()?
        };

        Ok(res)
    }
}

const INSERT: &str = r#"
INSERT INTO attachment (
    usage_type,
    display,
    description,
    content_type,
    length,
    sha2,
    file_url
) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id"#;

pub(crate) async fn insert_attachment(conn: &PgPool, att: &Attachment) -> Result<i32, MyError> {
    let display = sqlx::types::Json(att.display_as_map());
    let description = match att.description_as_map() {
        Some(x) => sqlx::types::Json(x.clone()),
        _ => sqlx::types::Json(EMPTY_LANGUAGE_MAP),
    };
    let file_url = att.file_url().map(|x| x.normalize().to_string());
    match sqlx::query_as::<_, RowID>(INSERT)
        .bind(att.usage_type().normalize().to_string())
        .bind(display)
        .bind(description)
        .bind(att.content_type().to_string())
        .bind(att.length())
        .bind(att.sha2())
        .bind(file_url)
        .fetch_one(conn)
        .await
    {
        Ok(x) => Ok(x.0),
        Err(x) => emit_db_error!(x, "Failed inserting Attachment ({})", att),
    }
}

const LINK_ATTACHMENT: &str = r#"
INSERT INTO attachments (statement_id, attachment_id) VALUES ($1, $2)"#;

/// Associate an Attachment to a Statement or SubStatement.
pub(crate) async fn link_attachment(
    conn: &PgPool,
    statement_id: i32,
    attachment_id: i32,
) -> Result<(), MyError> {
    match sqlx::query(LINK_ATTACHMENT)
        .bind(statement_id)
        .bind(attachment_id)
        .execute(conn)
        .await
    {
        Ok(_) => Ok(()),
        Err(x) => emit_db_error!(
            x,
            "Failed linking Attachment #{} to Statement #{}",
            attachment_id,
            statement_id
        ),
    }
}

const FIND: &str = r#"SELECT * FROM attachment WHERE id = $1"#;

#[instrument(skip(conn))]
pub(crate) async fn find_attachment(conn: &PgPool, id: i32) -> Result<Attachment, MyError> {
    debug!("id = {}", id);
    match sqlx::query_as::<_, TAttachment>(FIND)
        .bind(id)
        .fetch_one(conn)
        .await
    {
        Ok(x) => x.try_into(),
        Err(x) => emit_db_error!(x, "Failed finding Attachment #{}", id),
    }
}

const FIND_ATTACHMENTS: &str = r#"SELECT * FROM attachments WHERE statement_id = $1"#;

#[instrument(skip(conn))]
pub(crate) async fn find_attachments(conn: &PgPool, sid: i32) -> Result<Vec<Attachment>, MyError> {
    debug!("sid = {}", sid);
    match sqlx::query_as::<_, TAttachments>(FIND_ATTACHMENTS)
        .bind(sid)
        .fetch_all(conn)
        .await
    {
        Ok(x) => {
            debug!("x = {:?}", x);
            let mut res = vec![];
            for y in x {
                let att = find_attachment(conn, y.attachment_id).await?;
                res.push(att);
            }
            debug!("res = {:?}", res);
            Ok(res)
        }
        Err(x) => emit_db_error!(x, "Failed finding Attachment(s) for Statement #{}", sid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LanguageMap, MyLanguageTag};
    use sqlx::types::Json;
    use std::str::FromStr;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn test_unmarshal() -> Result<(), MyError> {
        let mut lm1 = LanguageMap::new();
        let en = MyLanguageTag::from_str("en")?;

        // lm1.insert_unchecked("en".to_owned(), "zDisplay".to_owned());
        lm1.insert(&en, "zDisplay");
        let display: Json<LanguageMap> = lm1.into();
        let mut lm2 = LanguageMap::new();
        // lm2.insert_unchecked("en".to_owned(), "zDescription".to_owned());
        lm2.insert(&en, "zDescription");
        let description: Option<Json<LanguageMap>> = Some(lm2.into());

        let row = TAttachment {
            id: 99,
            usage_type: "http://nowhere.net/attachment-usage/test".to_owned(),
            display,
            description,
            content_type: "text/html".to_owned(),
            length: 100,
            sha2: "495395e777cd98da653df9615d09c0fd6bb2f8d4788394cd53c56a3bfdcd848a".to_owned(),
            file_url: Some(
                "https://localhost/xapi//static/c44/sAZH2_GCudIGDdvf0xgHtLA/a1".to_owned(),
            ),
        };
        let maybe_att: Result<Attachment, _> = row.try_into();
        let att = maybe_att?;
        assert_eq!(att.length(), 100);

        Ok(())
    }
}
