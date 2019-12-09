use anyhow::{Context, Result};
use serde;

/// Append a HTML comment to the content of the message containing the metadata as json
pub struct HtmlCommentMetadataHandler {
    pub metadata_id: String,
}

impl HtmlCommentMetadataHandler {
    fn prefix(&self) -> String {
        format!("\n\n<!-- {}", self.metadata_id)
    }

    fn suffix(&self) -> String {
        format!(" -->")
    }

    pub fn add_metadata_to_comment<T: std::fmt::Display, M: serde::Serialize>(
        &self,
        comment: &T,
        metadata: &M,
    ) -> Result<String> {
        serde_json::to_string(&metadata)
            .context("Failed to serialize metadata")
            .map(|metadata_json| format!("{}{}{}{}",comment, self.prefix(), metadata_json, self.suffix()))
    }

    pub fn get_metadata_from_comment<M: serde::de::DeserializeOwned>(
        &self,
        comment: &str,
    ) -> Option<Result<M>> {
        let prefix = &self.prefix();
        let position: Option<(usize, usize)> = comment.find(prefix).and_then(|start|{
            let meta_start = start + prefix.len();
            let end = comment.find(&self.suffix());
            end.map(|e| (meta_start, e))
        });
        if let Some((start,end)) = position {
            Some(serde_json::from_str(&comment[start..end]).context("Failed to parse metadata"))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HtmlCommentMetadataHandler;

    #[test]
    fn test_add_get_metadata() {
        let metadata = vec![1, 2];
        let comment = "Some comment";
        let metadata_handler = HtmlCommentMetadataHandler {
            metadata_id: "aaaa".to_string(),
        };
        let expected_full_com = "Some comment\n\n<!-- aaaa[1,2] -->";

        assert_eq!(expected_full_com, &metadata_handler.add_metadata_to_comment(&comment, &metadata).unwrap());
        assert_eq!(&metadata, &metadata_handler.get_metadata_from_comment::<Vec<u64>>(expected_full_com).unwrap().unwrap());
        assert!(metadata_handler.get_metadata_from_comment::<()>(comment).is_none());
    }
}
