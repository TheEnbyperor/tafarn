table! {
    session (id) {
        id -> Uuid,
        access_token -> Varchar,
        expires_at -> Nullable<Timestamp>,
        refresh_token -> Nullable<Varchar>,
        claims -> Varchar,
    }
}

table! {
    apps (id) {
          id -> Uuid,
          name -> Varchar,
          website -> Nullable<Text>,
          redirect_uri -> Varchar,
          client_secret -> Varchar,
    }
}

table! {
    app_scopes (app_id, scope) {
          app_id -> Uuid,
          scope -> Varchar,
    }
}

table! {
    oauth_consents (id) {
        id -> Uuid,
        app_id -> Uuid,
        user_id -> Varchar,
        time -> Timestamp,
    }
}

table! {
    oauth_consent_scopes (consent_id, scope) {
        consent_id -> Uuid,
        scope -> Varchar,
    }
}

table! {
    oauth_codes (id) {
        id -> Uuid,
        time -> Timestamp,
        redirect_uri -> Varchar,
        client_id -> Uuid,
        user_id -> Varchar,
    }
}

table! {
    oauth_code_scopes (code_id, scope) {
        code_id -> Uuid,
        scope -> Varchar,
    }
}

table! {
    oauth_token (id) {
        id -> Uuid,
        time -> Timestamp,
        client_id -> Uuid,
        user_id -> Varchar,
        revoked -> Bool,
    }
}

table! {
    oauth_token_scopes (token_id, scope) {
        token_id -> Uuid,
        scope -> Varchar,
    }
}

table! {
    accounts (id) {
        id -> Uuid,
        iid -> Int4,
        actor -> Nullable<Varchar>,
        username -> Varchar,
        display_name -> Varchar,
        bio -> Varchar,
        locked -> Bool,
        bot -> Bool,
        group -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        default_sensitive -> Nullable<Bool>,
        default_language -> Nullable<Varchar>,
        discoverable -> Nullable<Bool>,
        follower_count -> Integer,
        following_count -> Integer,
        statuses_count -> Integer,
        owned_by -> Nullable<Varchar>,
        private_key -> Nullable<Varchar>,
        local -> Bool,
        inbox_url -> Nullable<Varchar>,
        outbox_url -> Nullable<Varchar>,
        shared_inbox_url -> Nullable<Varchar>,
        url -> Nullable<Varchar>,
        avatar_file -> Nullable<Varchar>,
        avatar_content_type -> Nullable<Varchar>,
        avatar_remote_url -> Nullable<Varchar>,
        header_file -> Nullable<Varchar>,
        header_content_type -> Nullable<Varchar>,
        header_remote_url -> Nullable<Varchar>,
        follower_collection_url -> Nullable<Varchar>,
    }
}

table! {
    account_fields (id) {
        id -> Uuid,
        account_id -> Uuid,
        name -> VarChar,
        value -> Varchar,
        sort_order -> Integer,
    }
}

table! {
    web_push_subscriptions (id) {
        id -> Uuid,
        token_id -> Uuid,
        account_id -> Uuid,
        endpoint -> Varchar,
        p256dh -> Varchar,
        auth -> Varchar,
        follow -> Bool,
        favourite -> Bool,
        reblog -> Bool,
        mention -> Bool,
        poll -> Bool,
        status -> Bool,
        follow_request -> Bool,
        update -> Bool,
        admin_sign_up -> Bool,
        admin_report -> Bool,
        policy -> Varchar,
    }
}

table! {
    public_keys (id) {
        id -> Uuid,
        key_id -> Varchar,
        user_id -> Uuid,
        key -> Varchar,
    }
}

table! {
    following (id) {
        id -> Uuid,
        iid -> Int4,
        follower -> Uuid,
        followee -> Uuid,
        created_at -> Timestamp,
        pending -> Bool,
    }
}

table! {
    notifications (id) {
        id -> Uuid,
        iid -> Int4,
        notification_type -> Varchar,
        account -> Uuid,
        cause -> Uuid,
        status -> Nullable<Uuid>,
        created_at -> Timestamp,
    }
}

table! {
    media (id) {
        id -> Uuid,
        media_type -> Varchar,
        file -> Nullable<Varchar>,
        content_type -> Nullable<Varchar>,
        remote_url -> Nullable<Varchar>,
        preview_file -> Nullable<Varchar>,
        preview_content_type -> Nullable<Varchar>,
        blurhash -> Nullable<Varchar>,
        focus_x -> Nullable<Float8>,
        focus_y -> Nullable<Float8>,
        original_width -> Nullable<Int4>,
        original_height -> Nullable<Int4>,
        preview_width -> Nullable<Int4>,
        preview_height -> Nullable<Int4>,
        created_at -> Timestamp,
        description -> Nullable<Varchar>,
        owned_by -> Nullable<Varchar>,
    }
}

table! {
    statuses (id) {
        id -> Uuid,
        iid -> Int4,
        url -> Varchar,
        uri -> Nullable<Varchar>,
        text -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        in_reply_to_id -> Nullable<Uuid>,
        boost_of_id -> Nullable<Uuid>,
        in_reply_to_url -> Nullable<Varchar>,
        boost_of_url -> Nullable<Varchar>,
        sensitive -> Bool,
        spoiler_text -> Varchar,
        language -> Nullable<Varchar>,
        local -> Bool,
        account_id -> Uuid,
        deleted_at -> Nullable<Timestamp>,
        edited_at -> Nullable<Timestamp>,
        public -> Bool,
        visible -> Bool,
    }
}

table! {
    status_media_attachments (status_id, media_attachment_id) {
        status_id -> Uuid,
        media_attachment_id -> Uuid,
        attachment_order -> Int4,
    }
}

table! {
    status_audiences (id) {
        id -> Uuid,
        status_id -> Uuid,
        mention -> Bool,
        account -> Nullable<Uuid>,
        account_followers -> Nullable<Uuid>,
    }
}

table! {
    home_timeline (id) {
        id -> Int4,
        account_id -> Uuid,
        status_id -> Uuid,
    }
}

table! {
    public_timeline (id) {
        id -> Int4,
        status_id -> Uuid,
    }
}

table! {
    likes (id) {
        id -> Uuid,
        iid -> Int4,
        status -> Nullable<Uuid>,
        account -> Uuid,
        created_at -> Timestamp,
        url -> Nullable<Varchar>,
        local -> Bool,
        status_url -> Nullable<Varchar>,
    }
}

table! {
    bookmarks (id) {
        id -> Uuid,
        iid -> Int4,
        status -> Uuid,
        account -> Uuid,
    }
}

table! {
    pins (id) {
        id -> Uuid,
        iid -> Int4,
        status -> Uuid,
        account -> Uuid,
    }
}

table! {
    account_notes (account, owner) {
        account -> Uuid,
        owner -> Uuid,
        note -> Varchar,
    }
}

joinable!(app_scopes -> apps (app_id));
joinable!(oauth_consent_scopes -> oauth_consents (consent_id));
joinable!(oauth_code_scopes -> oauth_codes (code_id));
joinable!(oauth_token_scopes -> oauth_token (token_id));
joinable!(account_fields -> accounts (account_id));
joinable!(web_push_subscriptions -> oauth_token (token_id));
joinable!(web_push_subscriptions -> accounts (account_id));
joinable!(public_keys -> accounts (user_id));
joinable!(notifications -> accounts (account));
joinable!(statuses -> accounts (account_id));
joinable!(status_media_attachments -> media (media_attachment_id));
joinable!(status_media_attachments -> statuses (status_id));
joinable!(status_audiences -> statuses (status_id));

allow_tables_to_appear_in_same_query!(
    session,
    apps,
    app_scopes,
    oauth_consents,
    oauth_consent_scopes,
    oauth_codes,
    oauth_code_scopes,
    oauth_token,
    oauth_token_scopes,
    accounts,
    account_fields,
    web_push_subscriptions,
    public_keys,
    following,
    notifications,
    media,
    statuses,
    status_audiences,
    status_media_attachments,
    home_timeline,
    public_timeline,
    likes,
    bookmarks,
    pins,
    account_notes
);