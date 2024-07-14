use gqlient::{Id, Query, Singleton, Variable};
use indoc::indoc;
use std::fmt::{self, Write};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetLabelName {
    label_id: Id,
    prefix: Option<String>,
}

impl GetLabelName {
    pub(crate) fn new(label_id: Id) -> GetLabelName {
        GetLabelName {
            label_id,
            prefix: None,
        }
    }

    fn label_id_varname(&self) -> String {
        match self.prefix {
            Some(ref prefix) => format!("{prefix}_label_id"),
            None => String::from("label_id"),
        }
    }
}

impl Query for GetLabelName {
    type Output = String;

    fn with_variable_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    fn write_graphql<W: Write>(&self, mut s: W) -> fmt::Result {
        writeln!(
            s,
            indoc! {"
            node(id: ${label_id_varname}) {{
                ... on Label {{
                    name
                }}
            }}
        "},
            label_id_varname = self.label_id_varname(),
        )
    }

    fn variables(&self) -> [(String, Variable); 1] {
        [(
            self.label_id_varname(),
            Variable {
                gql_type: String::from("ID!"),
                value: self.label_id.clone().into(),
            },
        )]
    }

    fn parse_response(&self, value: serde_json::Value) -> Result<Self::Output, serde_json::Error> {
        serde_json::from_value::<Singleton<String>>(value).map(|n| n.0)
    }
}
