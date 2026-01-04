/// Macros to reduce boilerplate in feed implementations across communities.
/// This helps standardize common patterns and reduce repetitive code.

/// Implement common CRUD operations for a Feed type.
/// This macro generates the standard all/get/delete implementations that are
/// nearly identical across all community feed types.
#[macro_export]
macro_rules! impl_feed_crud {
    ($feed_type:ty, $model_type:ty, $table:ident, $community_name:expr) => {
        fn all(db: Database) -> Result<Vec<Self>>
        where
            Self: Sized,
        {
            use bottle_core::schema::$table::dsl::*;
            $table
                .load::<$model_type>(db)?
                .into_iter()
                .map(Self::try_from)
                .collect()
        }

        fn get(db: Database, feed_id: i32) -> Result<Option<Self>>
        where
            Self: Sized,
        {
            use bottle_core::schema::$table::dsl::*;
            let result = $table
                .filter(id.eq(feed_id))
                .first::<$model_type>(db)
                .optional()?;
            result.map(Self::try_from).transpose()
        }

        fn delete(db: Database, feed_id: i32) -> Result<()>
        where
            Self: Sized,
        {
            use bottle_core::schema::$table::dsl::*;
            diesel::delete($table.filter(id.eq(feed_id))).execute(db)?;
            tracing::info!("Deleted {} feed {}", $community_name, feed_id);
            Ok(())
        }
    };
}

/// Implement common CRUD operations for an Account type.
#[macro_export]
macro_rules! impl_account_crud {
    ($account_type:ty, $model_type:ty, $table:ident, $community_name:expr) => {
        fn all(db: Database) -> Result<Vec<Self>>
        where
            Self: Sized,
        {
            use bottle_core::schema::$table::dsl::*;
            let results = $table
                .load::<$model_type>(db)?
                .into_iter()
                .map(Self::from)
                .collect();
            Ok(results)
        }

        fn get(db: Database, account_id: i32) -> Result<Option<Self>>
        where
            Self: Sized,
        {
            use bottle_core::schema::$table::dsl::*;
            let result = $table
                .filter(id.eq(account_id))
                .first::<$model_type>(db)
                .optional()?;
            Ok(result.map(Self::from))
        }

        fn delete(db: Database, account_id: i32) -> Result<()>
        where
            Self: Sized,
        {
            use bottle_core::schema::$table::dsl::*;
            diesel::delete($table.filter(id.eq(account_id))).execute(db)?;
            tracing::info!("Deleted {} account {}", $community_name, account_id);
            Ok(())
        }
    };
}

/// Generate a FeedView from feed parameters.
#[macro_export]
macro_rules! impl_feed_view {
    ($self:expr, $feed_id:expr, $community:expr, $name:expr, $watching:expr, $description:expr) => {
        FeedView {
            feed_id: $feed_id,
            community: $community.to_string(),
            name: $name,
            watching: $watching,
            description: $description,
        }
    };
}

/// Implement the modify method for a feed with standard update pattern.
#[macro_export]
macro_rules! impl_feed_modify {
    ($self:expr, $db:expr, $info:expr, $table:ident, $update_type:ty, $community_name:expr) => {{
        use bottle_core::schema::$table;
        let update = <$update_type>::from($info);
        diesel::update($table::table.find($self.id))
            .set(&update)
            .execute($db)?;
        $self.name = $info.name.clone();
        $self.watching = $info.watching;
        $self.first_fetch_limit = $info.first_fetch_limit;
        tracing::info!("Modified {} feed {}: {:?}", $community_name, $self.id, $info);
        Ok($self.view())
    }};
}
