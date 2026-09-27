use loco_rs::{app::AppContext, testing::prelude::*};
use loco_todo::{
    app::App,
    models::todos::{self, Filter},
};
use sea_orm::IntoActiveModel;
use serial_test::serial;

async fn titles(ctx: &AppContext, filter: Filter) -> Vec<(String, bool)> {
    todos::Model::list(&ctx.db, filter)
        .await
        .unwrap()
        .into_iter()
        .map(|t| (t.title, t.completed))
        .collect()
}

#[tokio::test]
#[serial]
async fn empty_list_hides_main_and_footer() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/").await;
        assert_eq!(res.status_code(), 200);
        let body = res.text();
        assert!(body.contains(r#"class="new-todo""#));
        assert!(!body.contains(r#"class="main""#));
        assert!(!body.contains(r#"class="footer""#));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_add_todo_and_title_is_trimmed() {
    request::<App, _, _>(|request, ctx| async move {
        let res = request
            .post("/todos")
            .form(&[("title", "  buy milk  "), ("filter", "all")])
            .await;
        assert_eq!(res.status_code(), 200);
        let body = res.text();
        assert!(body.contains(r#"id="todo-list-container""#));
        assert!(body.contains(">buy milk</label>"));
        assert!(body.contains("<strong>1</strong> item left"));
        assert_eq!(
            titles(&ctx, Filter::All).await,
            vec![("buy milk".into(), false)]
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn blank_todo_is_rejected() {
    request::<App, _, _>(|request, ctx| async move {
        let res = request.post("/todos").form(&[("title", "   ")]).await;
        assert_eq!(res.status_code(), 400);
        assert!(titles(&ctx, Filter::All).await.is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn titles_are_html_escaped() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.post("/todos").form(&[("title", "<b>x</b>")]).await;
        let body = res.text();
        assert!(!body.contains("<b>x</b>"), "title must be escaped: {body}");
        assert!(body.contains("&lt;b&gt;x"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_toggle_todo() {
    request::<App, _, _>(|request, ctx| async move {
        let todo = todos::Model::create(&ctx.db, "a").await.unwrap();
        let res = request
            .post(&format!("/todos/{}/toggle", todo.id))
            .form(&[("filter", "all")])
            .await;
        assert_eq!(res.status_code(), 200);
        assert!(res.text().contains(r#"class="completed""#));
        assert!(res.text().contains("Clear completed"));
        assert_eq!(titles(&ctx, Filter::All).await, vec![("a".into(), true)]);

        request
            .post(&format!("/todos/{}/toggle", todo.id))
            .form(&[("filter", "all")])
            .await;
        assert_eq!(titles(&ctx, Filter::All).await, vec![("a".into(), false)]);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn toggle_all_completes_all_then_reopens_all() {
    request::<App, _, _>(|request, ctx| async move {
        let a = todos::Model::create(&ctx.db, "a").await.unwrap();
        todos::Model::create(&ctx.db, "b").await.unwrap();
        a.into_active_model().toggle(&ctx.db).await.unwrap();

        let res = request
            .post("/todos/toggle_all")
            .form(&[("filter", "all")])
            .await;
        assert_eq!(res.status_code(), 200);
        assert!(res.text().contains("<strong>0</strong> items left"));
        assert_eq!(
            todos::Model::count(&ctx.db, Filter::Active).await.unwrap(),
            0
        );

        request
            .post("/todos/toggle_all")
            .form(&[("filter", "all")])
            .await;
        assert_eq!(
            todos::Model::count(&ctx.db, Filter::Completed)
                .await
                .unwrap(),
            0
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_edit_title_and_empty_title_deletes() {
    request::<App, _, _>(|request, ctx| async move {
        let todo = todos::Model::create(&ctx.db, "old").await.unwrap();
        let res = request
            .patch(&format!("/todos/{}", todo.id))
            .form(&[("title", " new "), ("filter", "all")])
            .await;
        assert_eq!(res.status_code(), 200);
        assert_eq!(titles(&ctx, Filter::All).await, vec![("new".into(), false)]);

        request
            .patch(&format!("/todos/{}", todo.id))
            .form(&[("title", "  "), ("filter", "all")])
            .await;
        assert!(titles(&ctx, Filter::All).await.is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_delete_todo() {
    request::<App, _, _>(|request, ctx| async move {
        let todo = todos::Model::create(&ctx.db, "a").await.unwrap();
        let res = request.delete(&format!("/todos/{}", todo.id)).await;
        assert_eq!(res.status_code(), 200);
        assert!(titles(&ctx, Filter::All).await.is_empty());

        let res = request.delete(&format!("/todos/{}", todo.id)).await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_clear_completed() {
    request::<App, _, _>(|request, ctx| async move {
        let a = todos::Model::create(&ctx.db, "a").await.unwrap();
        todos::Model::create(&ctx.db, "b").await.unwrap();
        a.into_active_model().toggle(&ctx.db).await.unwrap();

        let res = request.delete("/todos/completed?filter=all").await;
        assert_eq!(res.status_code(), 200);
        assert!(!res.text().contains("Clear completed"));
        assert_eq!(titles(&ctx, Filter::All).await, vec![("b".into(), false)]);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn filters_show_matching_todos_and_keep_the_filter() {
    request::<App, _, _>(|request, ctx| async move {
        let done = todos::Model::create(&ctx.db, "done-item").await.unwrap();
        todos::Model::create(&ctx.db, "open-item").await.unwrap();
        done.into_active_model().toggle(&ctx.db).await.unwrap();

        let active = request.get("/active").await.text();
        assert!(active.contains("open-item") && !active.contains("done-item"));
        assert!(active.contains(r#"<a href="/active" class="selected">"#));

        let completed = request.get("/completed").await.text();
        assert!(completed.contains("done-item") && !completed.contains("open-item"));

        let all = request.get("/").await.text();
        assert!(all.contains("done-item") && all.contains("open-item"));

        // A mutation made while filtered re-renders the same filter.
        let res = request
            .post("/todos")
            .form(&[("title", "another"), ("filter", "completed")])
            .await;
        let body = res.text();
        assert!(body.contains(r#"value="completed""#));
        assert!(!body.contains(">another</label>"));
        assert!(body.contains("<strong>2</strong> items left"));
    })
    .await;
}
