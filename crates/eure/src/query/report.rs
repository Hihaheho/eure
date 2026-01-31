use std::sync::Arc;

use query_flow::{Cachable, Db, Query, QueryResultExt as _, query};

use crate::report::{ErrorReports, format_error_reports};

#[query(debug = "{Self}({query:?})")]
pub fn with_formatted_error<T>(
    db: &impl Db,
    query: T,
    styled: bool,
) -> Result<Result<Arc<T::Output>, String>, query_flow::QueryError>
where
    T: Query + std::fmt::Debug + Cachable,
    T::Output: PartialEq,
{
    match db.query(query).downcast_err::<ErrorReports>()? {
        Ok(output) => Ok(Ok(output)), // Query succeeded
        Err(e) => Ok(Err(format_error_reports(db, e.get(), styled)?)),
    }
}
