#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
use chrono::{DateTime, Datelike, Duration, Local, TimeZone};
use clap::Parser;
use itertools::Itertools;
use reqwest::{
    blocking::Client,
    header::{HeaderMap, ACCEPT, COOKIE},
    StatusCode,
};
use securestore::{KeySource, SecretsManager};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{collections::HashMap, fs::read_to_string};

const YEAR: i32 = 2025;
const LEADERBOARDS: [i32; 2] = [649_161, 1_027_450];
const CACHEFILE: &str = ".aoc.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Star {
    get_star_ts: i64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Member {
    // global_score: i32,
    name: Option<String>,
    stars: i32,
    id: i32,
    last_star_ts: i64,
    local_score: i32,
    completion_day_level: HashMap<u32, HashMap<u32, Star>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Aoc {
    event: String,
    owner_id: i32,
    members: HashMap<String, Member>,
}

#[derive(Deserialize, Serialize)]
struct CacheEntry {
    timestamp: DateTime<Local>,
    data: Aoc,
}
type Cache = HashMap<i32, CacheEntry>;

struct Report {
    timestamp: DateTime<Local>,
    elapsed: Duration,
    member: String,
    star: String,
}

fn get_json(leaderbord: i32, flush_cache: bool) -> Aoc {
    let mut cache = Cache::new();
    if !flush_cache && std::path::Path::new(CACHEFILE).exists() {
        cache = serde_json::from_str(&read_to_string(CACHEFILE).unwrap()).unwrap();
        if cache.contains_key(&leaderbord) {
            let entry = cache.get(&leaderbord).unwrap();
            if entry.timestamp + Duration::minutes(15) > Local::now() {
                println!("using cache");
                return entry.data.clone();
            }
        }
    }
    println!("fetchin data");
    let client = Client::new();
    let url = format!(
        "https://adventofcode.com/{}/leaderboard/private/view/{}.json",
        YEAR, leaderbord
    );
    let key_path = Path::new(".secrets.key");
    let sman = SecretsManager::load("secrets.json", KeySource::Path(key_path))
        .expect("Failed to load secrets");
    let session = sman.get("session").expect("Couldn't get session cookie");
    let mut headers = HeaderMap::new();
    headers.insert(COOKIE, format!("session={};", session).parse().unwrap());
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    let res = client.get(url).headers(headers).send().unwrap();
    if res.status() != StatusCode::OK {
        println!("Fetch failed, cookie probably outdated.");
        println!(
            "Set a new cookie with 'ssclient set session <COOKIE>' ('cargo install ssclient')."
        );
        std::process::exit(1);
    }
    let text = res.text().unwrap();
    let aoc: Aoc = serde_json::from_str(&text).unwrap();
    cache.insert(
        leaderbord,
        CacheEntry {
            timestamp: Local::now(),
            data: aoc,
        },
    );
    std::fs::write(CACHEFILE, serde_json::to_string(&cache).unwrap()).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn duration_string(d: Duration) -> String {
    if d.num_days() > 0 {
        format!(
            "{}d {}:{:02}:{:02}",
            d.num_days(),
            d.num_hours() % 24,
            d.num_minutes() % 60,
            d.num_seconds() % 60
        )
    } else if d.num_hours() > 0 {
        format!(
            "{}:{:02}:{:02}",
            d.num_hours(),
            d.num_minutes() % 60,
            d.num_seconds() % 60
        )
    } else {
        format!("{:02}:{:02}", d.num_minutes() % 60, d.num_seconds() % 60)
    }
}

fn timeline(members: &HashMap<String, Member>) -> Vec<Report> {
    let mut timeline = Vec::<Report>::new();
    for member in members.values() {
        for dayno in member.completion_day_level.keys().sorted() {
            let day = &member.completion_day_level[dayno];
            let mut start = Local
                .with_ymd_and_hms(YEAR, 12, *dayno, 6, 0, 0)
                .single()
                .unwrap();
            for star in 1..=2 {
                if day.contains_key(&star) {
                    let solvetime = Local
                        .timestamp_opt(day[&star].get_star_ts, 0)
                        .single()
                        .unwrap();
                    timeline.push(Report {
                        timestamp: solvetime,
                        elapsed: solvetime - start,
                        member: if let Some(name) = member.name.clone() {
                            name
                        } else {
                            format!("Anonymous#{}", member.id)
                        },
                        star: format!("{:02}-{}", dayno, star),
                    });
                    start = solvetime;
                }
            }
        }
    }
    timeline.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    timeline
}

fn report(leaderbord: i32, all: bool, flush_cache: bool) {
    println!("\n{}", String::from_utf8(vec![b'#'; 70]).unwrap());
    let aoc = get_json(leaderbord, flush_cache);
    let max_score = aoc.members.len();

    let mut day = String::new();
    let mut score: HashMap<String, usize> = HashMap::new();
    let mut total_score: HashMap<String, usize> = HashMap::new();
    let today = chrono::offset::Local::now().day();

    for event in timeline(&aoc.members) {
        let event_day = format!("{}", event.timestamp.format("%B %e"));

        score
            .entry(event.star.clone())
            .and_modify(|e| *e -= 1)
            .or_insert(max_score);

        let star_score = score[&event.star];
        if all || event.timestamp.day() == today {
            if event_day != day {
                println!("\n{}", event_day);
                day = event_day;
            }
            println!(
                "  {} {:25}\t{} [{}] ({})",
                event.timestamp.time(),
                event.member,
                event.star,
                star_score,
                duration_string(event.elapsed)
            );
        }

        total_score
            .entry(event.member.clone())
            .and_modify(|e| *e += star_score)
            .or_insert(star_score);
    }
    println!("\nLeaderboard:");
    for (name, total) in total_score.iter().sorted_by(|a, b| b.1.cmp(a.1)) {
        println!("  {:25} {}", name, total);
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long, action)]
    all: bool,
    #[arg(short, long, action)]
    flush_cache: bool,
}

fn main() {
    let args = Cli::parse();
    for leaderbord in LEADERBOARDS {
        report(leaderbord, args.all, args.flush_cache);
    }
}
