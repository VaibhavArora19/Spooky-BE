use crate::{db::db::User, AppState};
use actix_web::{HttpResponse, post, web};
use mongodb::{bson::doc, bson::oid::ObjectId};
use names::Generator;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CreateUserRequest {
    name: String,
    avatar: String,
}

#[derive(Serialize, Deserialize)]
pub struct AddNewUserRequest {
    lobby_id: String,
    user_id: String,
}

#[post("/user/create")]
pub async fn create_new_user(
    req: web::Json<CreateUserRequest>,
    app_state: web::Data<AppState>,
) -> HttpResponse {

    let mut name_generator = Generator::with_naming(names::Name::Numbered);
    let username = name_generator.next().unwrap();

    let user_collection = app_state.db.collection::<User>("users");

    let user_id = ObjectId::new();

    let user = User {
        id: Some(user_id.clone()),
        username: username.clone(),
        name: req.name.clone(),
        avatar: req.avatar.clone(),
    };

    match user_collection.insert_one(user.clone()).await {
        Ok(insert_info) => {
            log::info!("User inserted successfully. Insert info: {:?}", insert_info);

            app_state.redis.clone()
                .set_nx::<String, String, ()>(
                    user_id.clone().to_string(),
                    serde_json::to_string(&user).unwrap(),
                )
                .await
                .unwrap();

            let user = User {
                id: Some(user_id),
                username: username,
                name: req.name.clone(),
                avatar: req.avatar.clone(),
            };

            HttpResponse::Ok().json(serde_json::to_string(&user).unwrap())
        }
        Err(error) => {
            log::error!(
                "Failed to insert user into the DB. Failed with error: {:?}",
                error
            );

            HttpResponse::InternalServerError().body("Failed to insert user into the DB")
        }
    }
}
