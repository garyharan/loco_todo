use serde::Serialize;

use crate::models::todos::{self, Filter};

#[derive(Debug, Serialize)]
pub struct TodoResponse {
    pub id: i64,
    pub title: String,
    pub completed: bool,
}

impl From<&todos::Model> for TodoResponse {
    fn from(todo: &todos::Model) -> Self {
        Self {
            id: todo.id,
            title: todo.title.clone(),
            completed: todo.completed,
        }
    }
}

/// Everything the TodoMVC page needs to render the list and its footer.
#[derive(Debug, Serialize)]
pub struct TodoListResponse {
    pub todos: Vec<TodoResponse>,
    pub filter: Filter,
    pub total_count: u64,
    pub active_count: u64,
    pub completed_count: u64,
}

impl TodoListResponse {
    #[must_use]
    pub fn new(
        todos: &[todos::Model],
        filter: Filter,
        total_count: u64,
        active_count: u64,
    ) -> Self {
        Self {
            todos: todos.iter().map(TodoResponse::from).collect(),
            filter,
            total_count,
            active_count,
            completed_count: total_count - active_count,
        }
    }
}
