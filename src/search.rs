// CRATES
use crate::utils::{cookie, error, param, template, val, Post, Preferences};
use crate::{client::json, RequestExt};
use askama::Template;
use hyper::{Body, Request, Response};

// STRUCTS
struct SearchParams {
	q: String,
	sort: String,
	t: String,
	before: String,
	after: String,
	restrict_sr: String,
}

// STRUCTS
struct Subreddit {
	name: String,
	url: String,
	description: String,
	subscribers: i64,
}

#[derive(Template)]
#[template(path = "search.html", escape = "none")]
struct SearchTemplate {
	posts: Vec<Post>,
	subreddits: Vec<Subreddit>,
	sub: String,
	params: SearchParams,
	prefs: Preferences,
}

// SERVICES
pub async fn find(req: Request<Body>) -> Result<Response<Body>, String> {
	let nsfw_results = if cookie(&req, "show_nsfw") == "on" { "&include_over_18=on" } else { "" };
	let path = format!("{}.json?{}{}", req.uri().path(), req.uri().query().unwrap_or_default(), nsfw_results);
	let sub = req.param("sub").unwrap_or_default();
	let query = param(&path, "q");

	let sort = if param(&path, "sort").is_empty() {
		"relevance".to_string()
	} else {
		param(&path, "sort")
	};

	let subreddits = if param(&path, "restrict_sr").is_empty() {
		search_subreddits(&query).await
	} else {
		Vec::new()
	};

	match Post::fetch(&path, String::new()).await {
		Ok((posts, after)) => template(SearchTemplate {
			posts,
			subreddits,
			sub,
			params: SearchParams {
				q: query.replace('"', "&quot;"),
				sort,
				t: param(&path, "t"),
				before: param(&path, "after"),
				after,
				restrict_sr: param(&path, "restrict_sr"),
			},
			prefs: Preferences::new(req),
		}),
		Err(msg) => error(req, msg).await,
	}
}

async fn search_subreddits(q: &str) -> Vec<Subreddit> {
	let subreddit_search_path = format!("/subreddits/search.json?q={}&limit=3", q.replace(' ', "+"));

	// Send a request to the url
	match json(subreddit_search_path).await {
		// If success, receive JSON in response
		Ok(response) => {
			match response["data"]["children"].as_array() {
				// For each subreddit from subreddit list
				Some(list) => list
					.iter()
					.map(|subreddit| Subreddit {
						name: val(subreddit, "display_name_prefixed"),
						url: val(subreddit, "url"),
						description: val(subreddit, "public_description"),
						subscribers: subreddit["data"]["subscribers"].as_f64().unwrap_or_default() as i64,
					})
					.collect::<Vec<Subreddit>>(),
				_ => Vec::new(),
			}
		}
		// If the Reddit API returns an error, exit this function
		_ => Vec::new(),
	}
}
