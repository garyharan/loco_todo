use loco_rs::prelude::*;
use sea_orm::{sea_query::Expr, QueryOrder};
use serde::{Deserialize, Serialize};

pub use super::_entities::todos::{ActiveModel, Column, Entity, Model};
pub type Todos = Entity;

/// Which slice of the list is being looked at — the `All / Active / Completed`
/// links in the footer.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Filter {
    #[default]
    All,
    Active,
    Completed,
}

impl Filter {
    fn condition(self) -> Option<Condition> {
        match self {
            Self::All => None,
            Self::Active => Some(Condition::all().add(Column::Completed.eq(false))),
            Self::Completed => Some(Condition::all().add(Column::Completed.eq(true))),
        }
    }
}

#[derive(Debug, Validate, Deserialize)]
pub struct Validator {
    #[validate(length(min = 1, message = "title must not be empty"))]
    pub title: String,
}

impl Validatable for ActiveModel {
    fn validator(&self) -> Box<dyn Validate> {
        Box::new(Validator {
            title: self.title.as_ref().clone(),
        })
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.validate()?;
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

// implement your read-oriented logic here
impl Model {
    /// # Errors
    /// Returns `ModelError::EntityNotFound` if no such todo exists.
    pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> ModelResult<Self> {
        Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Todos matching `filter`, oldest first.
    ///
    /// # Errors
    /// Returns a `ModelError` on database failure.
    pub async fn list(db: &DatabaseConnection, filter: Filter) -> ModelResult<Vec<Self>> {
        let mut query = Entity::find();
        if let Some(condition) = filter.condition() {
            query = query.filter(condition);
        }
        Ok(query.order_by_asc(Column::Id).all(db).await?)
    }

    /// Number of todos matching `filter`.
    ///
    /// # Errors
    /// Returns a `ModelError` on database failure.
    pub async fn count(db: &DatabaseConnection, filter: Filter) -> ModelResult<u64> {
        let mut query = Entity::find();
        if let Some(condition) = filter.condition() {
            query = query.filter(condition);
        }
        Ok(query.count(db).await?)
    }

    /// Adds a new, incomplete todo. The title is trimmed; an empty title fails
    /// validation.
    ///
    /// # Errors
    /// Returns `ModelError::Validation` for a blank title.
    pub async fn create(db: &DatabaseConnection, title: &str) -> ModelResult<Self> {
        let todo = ActiveModel {
            title: ActiveValue::Set(title.trim().to_string()),
            completed: ActiveValue::Set(false),
            ..Default::default()
        };
        // Validate up front: an error raised from `before_save` reaches us as a
        // `DbErr` (a 500), whereas `ModelError::Validation` renders as a 400.
        todo.validate().map_err(ModelError::Validation)?;
        Ok(todo.insert(db).await?)
    }

    /// Marks every todo complete — or, if they already all are, marks every
    /// todo active again. This is the "toggle all" chevron.
    ///
    /// # Errors
    /// Returns a `ModelError` on database failure.
    pub async fn toggle_all(db: &DatabaseConnection) -> ModelResult<()> {
        let all_completed = Self::count(db, Filter::Active).await? == 0;
        Entity::update_many()
            .col_expr(Column::Completed, Expr::value(!all_completed))
            .col_expr(Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .exec(db)
            .await?;
        Ok(())
    }

    /// Deletes every completed todo.
    ///
    /// # Errors
    /// Returns a `ModelError` on database failure.
    pub async fn clear_completed(db: &DatabaseConnection) -> ModelResult<()> {
        Entity::delete_many()
            .filter(Column::Completed.eq(true))
            .exec(db)
            .await?;
        Ok(())
    }
}

// implement your write-oriented logic here
impl ActiveModel {
    /// Flips the todo between active and completed.
    ///
    /// # Errors
    /// Returns a `ModelError` on database failure.
    pub async fn toggle(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        let completed = *self.completed.as_ref();
        self.completed = ActiveValue::Set(!completed);
        Ok(self.update(db).await?)
    }

    /// Saves an edited title. Per TodoMVC, clearing the title deletes the todo,
    /// so this returns `None` when the todo is gone.
    ///
    /// # Errors
    /// Returns a `ModelError` on database failure.
    pub async fn rename(
        mut self,
        db: &DatabaseConnection,
        title: &str,
    ) -> ModelResult<Option<Model>> {
        let title = title.trim();
        if title.is_empty() {
            self.delete(db).await?;
            return Ok(None);
        }
        self.title = ActiveValue::Set(title.to_string());
        Ok(Some(self.update(db).await?))
    }
}

// implement your custom finders, selectors oriented logic here
impl Entity {}
