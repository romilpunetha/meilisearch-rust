use crate::{errors::Error, indexes::Index};
use either::Either;
use serde::{de::DeserializeOwned, Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct MatchRange {
    pub start: usize,
    pub length: usize,
}

#[derive(Serialize, Debug, Eq, PartialEq, Clone)]
#[serde(transparent)]
pub struct Filter<'a> {
    #[serde(with = "either::serde_untagged")]
    inner: Either<&'a str, Vec<&'a str>>,
}

impl<'a> Filter<'a> {
    pub fn new(inner: Either<&'a str, Vec<&'a str>>) -> Filter {
        Filter { inner }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum MatchingStrategies {
    #[serde(rename = "all")]
    ALL,
    #[serde(rename = "last")]
    LAST,
}

/// A single result.
/// Contains the complete object, optionally the formatted object, and optionally an object that contains information about the matches.
#[derive(Deserialize, Debug)]
pub struct SearchResult<T> {
    /// The full result.
    #[serde(flatten)]
    pub result: T,
    /// The formatted result.
    #[serde(rename = "_formatted")]
    pub formatted_result: Option<Map<String, Value>>,
    /// The object that contains information about the matches.
    #[serde(rename = "_matchesPosition")]
    pub matches_position: Option<HashMap<String, Vec<MatchRange>>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
/// A struct containing search results and other information about the search.
pub struct SearchResults<T> {
    /// Results of the query
    pub hits: Vec<SearchResult<T>>,
    /// Number of documents skipped
    pub offset: usize,
    /// Number of results returned
    pub limit: usize,
    /// Total number of matches
    pub estimated_total_hits: usize,
    /// Distribution of the given facets
    pub facet_distribution: Option<HashMap<String, HashMap<String, usize>>>,
    /// Processing time of the query
    pub processing_time_ms: usize,
    /// Query originating the response
    pub query: String,
}

fn serialize_with_wildcard<S: Serializer, T: Serialize>(
    data: &Option<Selectors<T>>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match data {
        Some(Selectors::All) => ["*"].serialize(s),
        Some(Selectors::Some(data)) => data.serialize(s),
        None => s.serialize_none(),
    }
}

fn serialize_attributes_to_crop_with_wildcard<S: Serializer>(
    data: &Option<Selectors<&[AttributeToCrop]>>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match data {
        Some(Selectors::All) => ["*"].serialize(s),
        Some(Selectors::Some(data)) => {
            let mut results = Vec::new();
            for (name, value) in data.iter() {
                let mut result = String::new();
                result.push_str(name);
                if let Some(value) = value {
                    result.push(':');
                    result.push_str(value.to_string().as_str());
                }
                results.push(result)
            }
            results.serialize(s)
        }
        None => s.serialize_none(),
    }
}

/// Some list fields in a `SearchQuery` can be set to a wildcard value.
/// This structure allows you to choose between the wildcard value and an exhaustive list of selectors.
#[derive(Debug, Clone)]
pub enum Selectors<T> {
    /// A list of selectors
    Some(T),
    /// The wildcard
    All,
}

type AttributeToCrop<'a> = (&'a str, Option<usize>);

/// A struct representing a query.
/// You can add search parameters using the builder syntax.
/// See [this page](https://docs.meilisearch.com/reference/features/search_parameters.html#query-q) for the official list and description of all parameters.
///
/// # Examples
///
/// ```
/// use serde::{Serialize, Deserialize};
/// # use meilisearch_sdk::{client::Client, search::SearchQuery, indexes::Index};
/// #
/// # let MEILISEARCH_URL = option_env!("MEILISEARCH_URL").unwrap_or("http://localhost:7700");
/// # let MEILISEARCH_API_KEY = option_env!("MEILISEARCH_API_KEY").unwrap_or("masterKey");
/// #
/// #[derive(Serialize, Deserialize, Debug)]
/// struct Movie {
///     name: String,
///     description: String,
/// }
///
/// # futures::executor::block_on(async move {
/// # let client = Client::new(MEILISEARCH_URL, MEILISEARCH_API_KEY);
/// # let index = client
/// #  .create_index("search_query_builder", None)
/// #  .await
/// #  .unwrap()
/// #  .wait_for_completion(&client, None, None)
/// #  .await.unwrap()
/// #  .try_make_index(&client)
/// #  .unwrap();
///
/// let mut res = SearchQuery::new(&index)
///     .with_query("space")
///     .with_offset(42)
///     .with_limit(21)
///     .execute::<Movie>()
///     .await
///     .unwrap();
///
/// assert_eq!(res.limit, 21);
/// # index.delete().await.unwrap().wait_for_completion(&client, None, None).await.unwrap();
/// # });
/// ```
///
/// ```
/// # use meilisearch_sdk::{client::Client, search::SearchQuery, indexes::Index};
/// #
/// # let MEILISEARCH_URL = option_env!("MEILISEARCH_URL").unwrap_or("http://localhost:7700");
/// # let MEILISEARCH_API_KEY = option_env!("MEILISEARCH_API_KEY").unwrap_or("masterKey");
/// #
/// # let client = Client::new(MEILISEARCH_URL, MEILISEARCH_API_KEY);
/// # let index = client.index("search_query_builder_build");
/// let query = index.search()
///     .with_query("space")
///     .with_offset(42)
///     .with_limit(21)
///     .build(); // you can also execute() instead of build()
/// ```
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery<'a> {
    #[serde(skip_serializing)]
    index: &'a Index,
    /// The text that will be searched for among the documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "q")]
    pub query: Option<&'a str>,
    /// The number of documents to skip.
    /// If the value of the parameter `offset` is `n`, the `n` first documents (ordered by relevance) will not be returned.
    /// This is helpful for pagination.
    ///
    /// Example: If you want to skip the first document, set offset to `1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// The maximum number of documents returned.
    /// If the value of the parameter `limit` is `n`, there will never be more than `n` documents in the response.
    /// This is helpful for pagination.
    ///
    /// Example: If you don't want to get more than two documents, set limit to `2`.
    /// Default: `20`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Filter applied to documents.
    /// Read the [dedicated guide](https://docs.meilisearch.com/reference/features/filtering.html) to learn the syntax.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter<'a>>,
    /// Facets for which to retrieve the matching count.
    ///
    /// Can be set to a [wildcard value](enum.Selectors.html#variant.All) that will select all existing attributes.
    /// Default: all attributes found in the documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_with_wildcard")]
    pub facets: Option<Selectors<&'a [&'a str]>>,
    /// Attributes to sort.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<&'a [&'a str]>,
    /// Attributes to display in the returned documents.
    ///
    /// Can be set to a [wildcard value](enum.Selectors.html#variant.All) that will select all existing attributes.
    /// Default: all attributes found in the documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_with_wildcard")]
    pub attributes_to_retrieve: Option<Selectors<&'a [&'a str]>>,
    /// Attributes whose values have to be cropped.
    /// Attributes are composed by the attribute name and an optional `usize` that overwrites the `crop_length` parameter.
    ///
    /// Can be set to a [wildcard value](enum.Selectors.html#variant.All) that will select all existing attributes.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_attributes_to_crop_with_wildcard")]
    pub attributes_to_crop: Option<Selectors<&'a [AttributeToCrop<'a>]>>,
    /// Maximum number of words including the matched query term(s) contained in the returned cropped value(s).
    /// See [attributes_to_crop](#structfield.attributes_to_crop).
    ///
    /// Default: `10`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_length: Option<usize>,
    /// Marker at the start and the end of a cropped value.
    /// ex: `...middle of a crop...`
    ///
    /// Default: `...`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop_marker: Option<&'a str>,
    /// Attributes whose values will contain **highlighted matching terms**.
    ///
    /// Can be set to a [wildcard value](enum.Selectors.html#variant.All) that will select all existing attributes.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_with_wildcard")]
    pub attributes_to_highlight: Option<Selectors<&'a [&'a str]>>,
    /// Tag in front of a highlighted term.
    /// ex: `<mytag>hello world`
    ///
    /// Default: `<em>`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_pre_tag: Option<&'a str>,
    /// Tag after the a highlighted term.
    /// ex: `hello world</ mytag>`
    ///
    /// Default: `</em>`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_post_tag: Option<&'a str>,
    /// Defines whether an object that contains information about the matches should be returned or not.
    ///
    /// Default: `false`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_matches_position: Option<bool>,

    /// Defines the strategy on how to handle queries containing multiple words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matching_strategy: Option<MatchingStrategies>,
}

#[allow(missing_docs)]
impl<'a> SearchQuery<'a> {
    pub fn new(index: &'a Index) -> SearchQuery<'a> {
        SearchQuery {
            index,
            query: None,
            offset: None,
            limit: None,
            filter: None,
            sort: None,
            facets: None,
            attributes_to_retrieve: None,
            attributes_to_crop: None,
            crop_length: None,
            crop_marker: None,
            attributes_to_highlight: None,
            highlight_pre_tag: None,
            highlight_post_tag: None,
            show_matches_position: None,
            matching_strategy: None,
        }
    }
    pub fn with_query<'b>(&'b mut self, query: &'a str) -> &'b mut SearchQuery<'a> {
        self.query = Some(query);
        self
    }

    pub fn with_offset<'b>(&'b mut self, offset: usize) -> &'b mut SearchQuery<'a> {
        self.offset = Some(offset);
        self
    }
    pub fn with_limit<'b>(&'b mut self, limit: usize) -> &'b mut SearchQuery<'a> {
        self.limit = Some(limit);
        self
    }
    pub fn with_filter<'b>(&'b mut self, filter: &'a str) -> &'b mut SearchQuery<'a> {
        self.filter = Some(Filter::new(Either::Left(filter)));
        self
    }
    pub fn with_array_filter<'b>(&'b mut self, filter: Vec<&'a str>) -> &'b mut SearchQuery<'a> {
        self.filter = Some(Filter::new(Either::Right(filter)));
        self
    }
    pub fn with_facets<'b>(
        &'b mut self,
        facets: Selectors<&'a [&'a str]>,
    ) -> &'b mut SearchQuery<'a> {
        self.facets = Some(facets);
        self
    }
    pub fn with_sort<'b>(&'b mut self, sort: &'a [&'a str]) -> &'b mut SearchQuery<'a> {
        self.sort = Some(sort);
        self
    }
    pub fn with_attributes_to_retrieve<'b>(
        &'b mut self,
        attributes_to_retrieve: Selectors<&'a [&'a str]>,
    ) -> &'b mut SearchQuery<'a> {
        self.attributes_to_retrieve = Some(attributes_to_retrieve);
        self
    }
    pub fn with_attributes_to_crop<'b>(
        &'b mut self,
        attributes_to_crop: Selectors<&'a [(&'a str, Option<usize>)]>,
    ) -> &'b mut SearchQuery<'a> {
        self.attributes_to_crop = Some(attributes_to_crop);
        self
    }
    pub fn with_crop_length<'b>(&'b mut self, crop_length: usize) -> &'b mut SearchQuery<'a> {
        self.crop_length = Some(crop_length);
        self
    }
    pub fn with_crop_marker<'b>(&'b mut self, crop_marker: &'a str) -> &'b mut SearchQuery<'a> {
        self.crop_marker = Some(crop_marker);
        self
    }
    pub fn with_attributes_to_highlight<'b>(
        &'b mut self,
        attributes_to_highlight: Selectors<&'a [&'a str]>,
    ) -> &'b mut SearchQuery<'a> {
        self.attributes_to_highlight = Some(attributes_to_highlight);
        self
    }
    pub fn with_highlight_pre_tag<'b>(
        &'b mut self,
        highlight_pre_tag: &'a str,
    ) -> &'b mut SearchQuery<'a> {
        self.highlight_pre_tag = Some(highlight_pre_tag);
        self
    }
    pub fn with_highlight_post_tag<'b>(
        &'b mut self,
        highlight_post_tag: &'a str,
    ) -> &'b mut SearchQuery<'a> {
        self.highlight_post_tag = Some(highlight_post_tag);
        self
    }
    pub fn with_show_matches_position<'b>(
        &'b mut self,
        show_matches_position: bool,
    ) -> &'b mut SearchQuery<'a> {
        self.show_matches_position = Some(show_matches_position);
        self
    }
    pub fn with_matching_strategy<'b>(
        &'b mut self,
        matching_strategy: MatchingStrategies,
    ) -> &'b mut SearchQuery<'a> {
        self.matching_strategy = Some(matching_strategy);
        self
    }
    pub fn build(&mut self) -> SearchQuery<'a> {
        self.clone()
    }
    /// Execute the query and fetch the results.
    pub async fn execute<T: 'static + DeserializeOwned>(
        &'a self,
    ) -> Result<SearchResults<T>, Error> {
        self.index.execute_query::<T>(self).await
    }
}

#[cfg(test)]
mod tests {
    use crate::{client::*, search::*};
    use meilisearch_test_macro::meilisearch_test;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Map, Value};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Nested {
        child: String,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Document {
        id: usize,
        value: String,
        kind: String,
        nested: Nested,
    }

    impl PartialEq<Map<String, Value>> for Document {
        fn eq(&self, rhs: &Map<String, Value>) -> bool {
            self.id.to_string() == rhs["id"]
                && self.value == rhs["value"]
                && self.kind == rhs["kind"]
        }
    }

    async fn setup_test_index(client: &Client, index: &Index) -> Result<(), Error> {
        let t0 = index.add_documents(&[
            Document { id: 0, kind: "text".into(), value: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.".to_string(), nested: Nested { child: "first".to_string() } },
            Document { id: 1, kind: "text".into(), value: "dolor sit amet, consectetur adipiscing elit".to_string(), nested: Nested { child: "second".to_string() } },
            Document { id: 2, kind: "title".into(), value: "The Social Network".to_string(), nested: Nested { child: "third".to_string() } },
            Document { id: 3, kind: "title".into(), value: "Harry Potter and the Sorcerer's Stone".to_string(), nested: Nested { child: "fourth".to_string() } },
            Document { id: 4, kind: "title".into(), value: "Harry Potter and the Chamber of Secrets".to_string(), nested: Nested { child: "fift".to_string() } },
            Document { id: 5, kind: "title".into(), value: "Harry Potter and the Prisoner of Azkaban".to_string(), nested: Nested { child: "sixth".to_string() } },
            Document { id: 6, kind: "title".into(), value: "Harry Potter and the Goblet of Fire".to_string(), nested: Nested { child: "seventh".to_string() } },
            Document { id: 7, kind: "title".into(), value: "Harry Potter and the Order of the Phoenix".to_string(), nested: Nested { child: "eighth".to_string() } },
            Document { id: 8, kind: "title".into(), value: "Harry Potter and the Half-Blood Prince".to_string(), nested: Nested { child: "ninth".to_string() } },
            Document { id: 9, kind: "title".into(), value: "Harry Potter and the Deathly Hallows".to_string(), nested: Nested { child: "tenth".to_string() } },
        ], None).await?;
        let t1 = index.set_filterable_attributes(["kind", "value"]).await?;
        let t2 = index.set_sortable_attributes(["title"]).await?;

        t2.wait_for_completion(client, None, None).await?;
        t1.wait_for_completion(client, None, None).await?;
        t0.wait_for_completion(client, None, None).await?;

        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_builder(_client: Client, index: Index) -> Result<(), Error> {
        let mut query = SearchQuery::new(&index);
        query.with_query("space").with_offset(42).with_limit(21);

        let res = query.execute::<Document>().await.unwrap();

        assert_eq!(res.query, "space".to_string());
        assert_eq!(res.limit, 21);
        assert_eq!(res.offset, 42);
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_string(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let results: SearchResults<Document> = index.search().with_query("dolor").execute().await?;
        assert_eq!(results.hits.len(), 2);
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_string_on_nested_field(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let results: SearchResults<Document> =
            index.search().with_query("second").execute().await?;

        assert_eq!(
            &Document {
                id: 1,
                value: "dolor sit amet, consectetur adipiscing elit".to_string(),
                kind: "text".to_string(),
                nested: Nested {
                    child: "second".to_string()
                }
            },
            &results.hits[0].result
        );

        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_limit(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let results: SearchResults<Document> = index.search().with_limit(5).execute().await?;
        assert_eq!(results.hits.len(), 5);
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_offset(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let results: SearchResults<Document> = index.search().with_offset(6).execute().await?;
        assert_eq!(results.hits.len(), 4);
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_filter(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let results: SearchResults<Document> = index
            .search()
            .with_filter("value = \"The Social Network\"")
            .execute()
            .await?;
        assert_eq!(results.hits.len(), 1);

        let results: SearchResults<Document> = index
            .search()
            .with_filter("NOT value = \"The Social Network\"")
            .execute()
            .await?;
        assert_eq!(results.hits.len(), 9);
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_filter_with_array(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let results: SearchResults<Document> = index
            .search()
            .with_array_filter(vec![
                "value = \"The Social Network\"",
                "value = \"The Social Network\"",
            ])
            .execute()
            .await?;
        assert_eq!(results.hits.len(), 1);

        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_facet_distribution(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let mut query = SearchQuery::new(&index);
        query.with_facets(Selectors::All);
        let results: SearchResults<Document> = index.execute_query(&query).await?;
        assert_eq!(
            results
                .facet_distribution
                .unwrap()
                .get("kind")
                .unwrap()
                .get("title")
                .unwrap(),
            &8
        );

        let mut query = SearchQuery::new(&index);
        query.with_facets(Selectors::Some(&["kind"]));
        let results: SearchResults<Document> = index.execute_query(&query).await?;
        assert_eq!(
            results
                .facet_distribution
                .clone()
                .unwrap()
                .get("kind")
                .unwrap()
                .get("title")
                .unwrap(),
            &8
        );
        assert_eq!(
            results
                .facet_distribution
                .unwrap()
                .get("kind")
                .unwrap()
                .get("text")
                .unwrap(),
            &2
        );
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_attributes_to_retrieve(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let results: SearchResults<Document> = index
            .search()
            .with_attributes_to_retrieve(Selectors::All)
            .execute()
            .await?;
        assert_eq!(results.hits.len(), 10);

        let mut query = SearchQuery::new(&index);
        query.with_attributes_to_retrieve(Selectors::Some(&["kind", "id"])); // omit the "value" field
        assert!(index.execute_query::<Document>(&query).await.is_err()); // error: missing "value" field
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_sort(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let mut query = SearchQuery::new(&index);
        query.with_query("harry potter");
        query.with_sort(&["title:desc"]);
        let results: SearchResults<Document> = index.execute_query(&query).await?;
        assert_eq!(results.hits.len(), 7);
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_attributes_to_crop(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let mut query = SearchQuery::new(&index);
        query.with_query("lorem ipsum");
        query.with_attributes_to_crop(Selectors::All);
        let results: SearchResults<Document> = index.execute_query(&query).await?;
        assert_eq!(
            &Document {
                id: 0,
                value: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do…"
                    .to_string(),
                kind: "text".to_string(),
                nested: Nested {
                    child: "first".to_string()
                }
            },
            results.hits[0].formatted_result.as_ref().unwrap()
        );

        let mut query = SearchQuery::new(&index);
        query.with_query("lorem ipsum");
        query.with_attributes_to_crop(Selectors::Some(&[("value", Some(5)), ("kind", None)]));
        let results: SearchResults<Document> = index.execute_query(&query).await?;
        assert_eq!(
            &Document {
                id: 0,
                value: "Lorem ipsum dolor sit amet…".to_string(),
                kind: "text".to_string(),
                nested: Nested {
                    child: "first".to_string()
                }
            },
            results.hits[0].formatted_result.as_ref().unwrap()
        );
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_crop_length(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let mut query = SearchQuery::new(&index);
        query.with_query("lorem ipsum");
        query.with_attributes_to_crop(Selectors::All);
        query.with_crop_length(200);
        let results: SearchResults<Document> = index.execute_query(&query).await?;
        assert_eq!(&Document {
            id: 0,
            value: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.".to_string(),
            kind: "text".to_string(),
            nested: Nested { child: "first".to_string() }
        },
        results.hits[0].formatted_result.as_ref().unwrap());

        let mut query = SearchQuery::new(&index);
        query.with_query("lorem ipsum");
        query.with_attributes_to_crop(Selectors::All);
        query.with_crop_length(5);
        let results: SearchResults<Document> = index.execute_query(&query).await?;
        assert_eq!(
            &Document {
                id: 0,
                value: "Lorem ipsum dolor sit amet…".to_string(),
                kind: "text".to_string(),
                nested: Nested {
                    child: "first".to_string()
                }
            },
            results.hits[0].formatted_result.as_ref().unwrap()
        );
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_customized_crop_marker(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let mut query = SearchQuery::new(&index);
        query.with_query("sed do eiusmod");
        query.with_attributes_to_crop(Selectors::All);
        query.with_crop_length(6);
        query.with_crop_marker("(ꈍᴗꈍ)");

        let results: SearchResults<Document> = index.execute_query(&query).await?;

        assert_eq!(
            &Document {
                id: 0,
                value: "(ꈍᴗꈍ) sed do eiusmod tempor incididunt ut(ꈍᴗꈍ)".to_string(),
                kind: "text".to_string(),
                nested: Nested {
                    child: "first".to_string()
                }
            },
            results.hits[0].formatted_result.as_ref().unwrap()
        );
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_customized_highlight_pre_tag(
        client: Client,
        index: Index,
    ) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let mut query = SearchQuery::new(&index);
        query.with_query("Social");
        query.with_attributes_to_highlight(Selectors::All);
        query.with_highlight_pre_tag("(⊃｡•́‿•̀｡)⊃ ");
        query.with_highlight_post_tag(" ⊂(´• ω •`⊂)");

        let results: SearchResults<Document> = index.execute_query(&query).await?;
        assert_eq!(
            &Document {
                id: 2,
                value: "The (⊃｡•́‿•̀｡)⊃ Social ⊂(´• ω •`⊂) Network".to_string(),
                kind: "title".to_string(),
                nested: Nested {
                    child: "third".to_string()
                }
            },
            results.hits[0].formatted_result.as_ref().unwrap()
        );

        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_attributes_to_highlight(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let mut query = SearchQuery::new(&index);
        query.with_query("dolor text");
        query.with_attributes_to_highlight(Selectors::All);
        let results: SearchResults<Document> = index.execute_query(&query).await?;
        assert_eq!(
            &Document {
                id: 1,
                value: "<em>dolor</em> sit amet, consectetur adipiscing elit".to_string(),
                kind: "<em>text</em>".to_string(),
                nested: Nested {
                    child: "first".to_string()
                }
            },
            results.hits[0].formatted_result.as_ref().unwrap(),
        );

        let mut query = SearchQuery::new(&index);
        query.with_query("dolor text");
        query.with_attributes_to_highlight(Selectors::Some(&["value"]));
        let results: SearchResults<Document> = index.execute_query(&query).await?;
        assert_eq!(
            &Document {
                id: 1,
                value: "<em>dolor</em> sit amet, consectetur adipiscing elit".to_string(),
                kind: "text".to_string(),
                nested: Nested {
                    child: "first".to_string()
                }
            },
            results.hits[0].formatted_result.as_ref().unwrap()
        );
        Ok(())
    }

    #[meilisearch_test]
    async fn test_query_show_matches_position(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let mut query = SearchQuery::new(&index);
        query.with_query("dolor text");
        query.with_show_matches_position(true);
        let results: SearchResults<Document> = index.execute_query(&query).await?;
        assert_eq!(results.hits[0].matches_position.as_ref().unwrap().len(), 2);
        assert_eq!(
            results.hits[0]
                .matches_position
                .as_ref()
                .unwrap()
                .get("value")
                .unwrap(),
            &vec![MatchRange {
                start: 0,
                length: 5
            }]
        );
        Ok(())
    }

    #[meilisearch_test]
    async fn test_phrase_search(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let mut query = SearchQuery::new(&index);
        query.with_query("harry \"of Fire\"");
        let results: SearchResults<Document> = index.execute_query(&query).await?;

        assert_eq!(results.hits.len(), 1);
        Ok(())
    }

    #[meilisearch_test]
    async fn test_matching_strategy_all(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let results = SearchQuery::new(&index)
            .with_query("Harry Styles")
            .with_matching_strategy(MatchingStrategies::ALL)
            .execute::<Document>()
            .await
            .unwrap();

        assert_eq!(results.hits.len(), 0);
        Ok(())
    }

    #[meilisearch_test]
    async fn test_matching_strategy_left(client: Client, index: Index) -> Result<(), Error> {
        setup_test_index(&client, &index).await?;

        let results = SearchQuery::new(&index)
            .with_query("Harry Styles")
            .with_matching_strategy(MatchingStrategies::LAST)
            .execute::<Document>()
            .await
            .unwrap();

        assert_eq!(results.hits.len(), 7);
        Ok(())
    }

    #[meilisearch_test]
    async fn test_generate_tenant_token_from_client(
        client: Client,
        index: Index,
    ) -> Result<(), Error> {
        use crate::key::{Action, KeyBuilder};

        setup_test_index(&client, &index).await?;

        let meilisearch_url = option_env!("MEILISEARCH_URL").unwrap_or("http://localhost:7700");
        let key = KeyBuilder::new()
            .with_action(Action::All)
            .with_index("*")
            .execute(&client)
            .await
            .unwrap();
        let allowed_client = Client::new(meilisearch_url, key.key);

        let search_rules = vec![
            json!({ "*": {}}),
            json!({ "*": Value::Null }),
            json!(["*"]),
            json!({ "*": { "filter": "kind = text" } }),
            json!([index.uid.to_string()]),
        ];

        for rules in search_rules {
            let token = allowed_client
                .generate_tenant_token(key.uid.clone(), rules, None, None)
                .expect("Cannot generate tenant token.");

            let new_client = Client::new(meilisearch_url, token.clone());

            let result: SearchResults<Document> = new_client
                .index(index.uid.to_string())
                .search()
                .execute()
                .await?;

            assert!(!result.hits.is_empty());
        }

        Ok(())
    }
}
