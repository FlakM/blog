use crate::DOMAIN;
use crate::{
    database::Repository, error::Error, hugo_posts::HugoBlogPost, objects::person::DbUser,
};
use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::{object::NoteType, public},
    protocol::{helpers::deserialize_one_or_many, verification::verify_domains_match},
    traits::Object,
};
use activitystreams_kinds::link::MentionType;
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use url::Url;

#[derive(Clone, Debug)]
pub struct FediPost {
    pub ap_id: ObjectId<FediPost>,
    pub creator: ObjectId<DbUser>,
    pub local: bool,
    pub blog_post: HugoBlogPost,
}

impl FediPost {
    pub fn render_content(&self) -> String {
        format!(
            r#"<p><b>{title}</b></p><p>{description}</p><p><a href="{url}">Read more</a></p>"#,
            title = self.blog_post.title,
            description = self.blog_post.description,
            url = self.blog_post.url,
        )
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    #[serde(rename = "type")]
    kind: NoteType,
    id: ObjectId<FediPost>,
    pub(crate) attributed_to: ObjectId<DbUser>,
    #[serde(deserialize_with = "deserialize_one_or_many")]
    pub(crate) to: Vec<Url>,
    content: String,
    in_reply_to: Option<ObjectId<FediPost>>,
    tag: Vec<MyCustomMention>,
}

/// Represents a hashtag mention accepted by Mastodon
#[derive(Debug, Clone, Deserialize)]
pub struct Hashtag(String);

impl Serialize for Hashtag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let name = &self.0;
        let hashtag = format!("#{}", name);
        let href = format!("https://{DOMAIN}/tags/{name}");
        let mut state = serializer.serialize_struct("Hashtag", 3)?;
        state.serialize_field("name", &hashtag)?;
        state.serialize_field("type", "Hashtag")?;
        state.serialize_field("href", &href)?;
        state.end()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Mention {
    pub href: Url,
    #[serde(rename = "type")]
    pub kind: MentionType,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum MyCustomMention {
    Hashtag(Hashtag),
    Mention(Mention),
}

#[async_trait::async_trait]
impl Object for FediPost {
    type DataType = Repository;
    type Kind = Note;
    type Error = Error;

    async fn read_from_id(
        _object_id: Url,
        _data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        Ok(None)
    }

    async fn from_json(
        _json: Self::Kind,
        _data: &Data<Self::DataType>,
    ) -> Result<Self, Self::Error> {
        unimplemented!()
    }

    async fn into_json(self, data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        let creator = self.creator.dereference(data).await?;
        let content = self.render_content();
        Ok(Note {
            kind: Default::default(),
            id: self.ap_id,
            attributed_to: creator.ap_id,
            to: vec![public()],
            content,
            in_reply_to: None,
            tag: self
                .blog_post
                .tags
                .unwrap_or_default()
                .iter()
                .map(|tag| MyCustomMention::Hashtag(Hashtag(tag.to_string())))
                .collect(),
        })
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain)?;
        Ok(())
    }
}
