#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
//! TodoMVC, server-rendered with HTMX.
//!
//! The three filter URLs (`/`, `/active`, `/completed`) render the full page.
//! Every mutation re-renders just the list and footer (`todos/_list.html`),
//! which HTMX swaps into `#todo-list-container`. The current filter rides along
//! with each request (a hidden `filter` input that the page `hx-include`s), so
//! the re-rendered list stays on the filter the user is looking at.
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::{
    models::todos::{self, Filter},
    views::todos::TodoListResponse,
};

#[derive(Debug, Default, Deserialize)]
pub struct FilterParams {
    #[serde(default)]
    pub filter: Filter,
}

#[derive(Debug, Deserialize)]
pub struct TitleParams {
    pub title: String,
    #[serde(default)]
    pub filter: Filter,
}

async fn render(
    v: &TeraView,
    ctx: &AppContext,
    filter: Filter,
    template: &str,
) -> Result<Response> {
    let items = todos::Model::list(&ctx.db, filter).await?;
    let total_count = todos::Model::count(&ctx.db, Filter::All).await?;
    let active_count = todos::Model::count(&ctx.db, Filter::Active).await?;
    format::render().view(
        v,
        template,
        data!({ "list": TodoListResponse::new(&items, filter, total_count, active_count) }),
    )
}

async fn render_page(v: &TeraView, ctx: &AppContext, filter: Filter) -> Result<Response> {
    render(v, ctx, filter, "todos/index.html").await
}

async fn render_list(v: &TeraView, ctx: &AppContext, filter: Filter) -> Result<Response> {
    render(v, ctx, filter, "todos/_list.html").await
}

#[debug_handler]
pub async fn all(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    render_page(&v, &ctx, Filter::All).await
}

#[debug_handler]
pub async fn active(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    render_page(&v, &ctx, Filter::Active).await
}

#[debug_handler]
pub async fn completed(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    render_page(&v, &ctx, Filter::Completed).await
}

#[debug_handler]
pub async fn create(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Form(params): Form<TitleParams>,
) -> Result<Response> {
    todos::Model::create(&ctx.db, &params.title).await?;
    render_list(&v, &ctx, params.filter).await
}

#[debug_handler]
pub async fn update(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Path(id): Path<i64>,
    Form(params): Form<TitleParams>,
) -> Result<Response> {
    let todo = todos::Model::find_by_id(&ctx.db, id).await?;
    todo.into_active_model()
        .rename(&ctx.db, &params.title)
        .await?;
    render_list(&v, &ctx, params.filter).await
}

#[debug_handler]
pub async fn toggle(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Path(id): Path<i64>,
    Form(params): Form<FilterParams>,
) -> Result<Response> {
    let todo = todos::Model::find_by_id(&ctx.db, id).await?;
    todo.into_active_model().toggle(&ctx.db).await?;
    render_list(&v, &ctx, params.filter).await
}

#[debug_handler]
pub async fn remove(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Path(id): Path<i64>,
    Query(params): Query<FilterParams>,
) -> Result<Response> {
    let todo = todos::Model::find_by_id(&ctx.db, id).await?;
    todo.delete(&ctx.db).await?;
    render_list(&v, &ctx, params.filter).await
}

#[debug_handler]
pub async fn toggle_all(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Form(params): Form<FilterParams>,
) -> Result<Response> {
    todos::Model::toggle_all(&ctx.db).await?;
    render_list(&v, &ctx, params.filter).await
}

#[debug_handler]
pub async fn clear_completed(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Query(params): Query<FilterParams>,
) -> Result<Response> {
    todos::Model::clear_completed(&ctx.db).await?;
    render_list(&v, &ctx, params.filter).await
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(all))
        .add("/active", get(active))
        .add("/completed", get(completed))
        .add("/todos", post(create))
        .add("/todos/toggle_all", post(toggle_all))
        .add("/todos/completed", delete(clear_completed))
        .add("/todos/{id}", patch(update).delete(remove))
        .add("/todos/{id}/toggle", post(toggle))
}
