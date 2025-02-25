use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::query_dsl::methods::LoadQuery;
use diesel::sql_types::BigInt;
use diesel::sqlite::Sqlite;

pub trait Paginate: Sized {
    fn paginate(self, page: i64, page_size: i64) -> Paginated<Self>;
}

impl<T> Paginate for T {
    /// Paginate a query with given page and page size.
    /// The resulting records will be `(T, i64)`, where the first element is the
    /// original query type, and the second element is the total number of records.
    fn paginate(self, page: i64, page_size: i64) -> Paginated<Self> {
        Paginated {
            query: self,
            limit: page_size,
            offset: page * page_size,
        }
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Paginated<T> {
    query: T,
    limit: i64,
    offset: i64,
}

impl<T> Paginated<T> {
    pub fn load_and_count<'a, U>(self, conn: &mut SqliteConnection) -> QueryResult<(Vec<U>, i64)>
    where
        Self: LoadQuery<'a, SqliteConnection, (U, i64)>,
    {
        let results = self.load::<(U, i64)>(conn)?;
        let total = results.first().map(|x| x.1).unwrap_or(0);
        let records = results.into_iter().map(|x| x.0).collect();
        Ok((records, total))
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T> RunQueryDsl<SqliteConnection> for Paginated<T> {}

impl<T> QueryFragment<Sqlite> for Paginated<T>
where
    T: QueryFragment<Sqlite>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.limit)?;
        out.push_sql(" OFFSET ");
        out.push_bind_param::<BigInt, _>(&self.offset)?;
        Ok(())
    }
}
