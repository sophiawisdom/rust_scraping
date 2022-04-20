#![feature(pattern)]
use std::{io::Write, time::SystemTime};
use serde;
use reqwest;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use tokio::{self};
use futures::{stream, StreamExt};
use scraper::{Html, Selector};
use indicatif::ProgressBar;

#[derive(Debug)]
struct CatInfo {
    category: String,
    page: i32,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct StoryInfo {
    member_id: u32,
    member_name: String,

    title: String,
    description: String,
    storyName: String,
    tags: Vec<String>,

    favorites: u32,
    views: u32,
    rating: u32, // in centi-rating points, so e.g. 3.45 = 345

    timeFetched: u64, // seconds since epoch
    // comments are more complicated; we should use https://classic.literotica.com/stories/storyfeedbackboard.php?id=343471&pagehint=6 or the like to get these.
}

const MAX_CONCURRENT_REQUESTS: usize = 1000;

mod parsers;

async fn fetch_and_parse(url: String, client: &ClientWithMiddleware) -> (scraper::Html, u128, u128) {
    let before_request = std::time::SystemTime::now();
    let resp = match client.get(&url).send().await {
        Ok(resp) => {
            match resp.bytes().await {
                Ok(bytes) => std::str::from_utf8(&bytes).unwrap().to_string(),
                Err(e) => panic!("getting bytes failed")
            }
        },
        Err(e) => {
            eprintln!("got error {e} on making request for page {}", &url);
            panic!("getting data failed")
        }
    };
    let after_request = std::time::SystemTime::now();
    let parsed = Html::parse_document(&resp);
    let after_parse = std::time::SystemTime::now();
    return (parsed, after_request.duration_since(before_request).unwrap().as_micros(), after_parse.duration_since(after_request).unwrap().as_micros());
}

#[tokio::main]
async fn main() {
    let reqwest_client = match reqwest::ClientBuilder::new()
    // Copied the user_agent from my browser which seems to convince it to let us through
    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.102 Safari/537.36")
    .gzip(true)
    .build() {
        Ok(client) => client,
        Err(error) => panic!("Couldn't make client: {:?}", error)
    };

    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let literotica_client = ClientBuilder::new(reqwest_client)
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .build();
    
    let category_links = vec!["https://www.literotica.com/c/anal-sex-stories".to_string(), "https://www.literotica.com/c/bdsm-stories".to_string(), "https://www.literotica.com/c/celebrity-stories".to_string(), "https://www.literotica.com/c/chain-stories".to_string(), "https://www.literotica.com/c/erotic-couplings".to_string(), "https://www.literotica.com/c/erotic-horror".to_string(), "https://www.literotica.com/c/exhibitionist-voyeur".to_string(), "https://www.literotica.com/c/fetish-stories".to_string(), "https://www.literotica.com/c/first-time-sex-stories".to_string(), "https://www.literotica.com/c/gay-sex-stories".to_string(), "https://www.literotica.com/c/group-sex-stories".to_string(), "https://www.literotica.com/c/adult-how-to".to_string(), "https://www.literotica.com/c/adult-humor".to_string(), "https://www.literotica.com/c/taboo-sex-stories".to_string(), "https://www.literotica.com/c/interracial-erotic-stories".to_string(), "https://www.literotica.com/c/lesbian-sex-stories".to_string(), "https://www.literotica.com/c/erotic-letters".to_string(), "https://www.literotica.com/c/loving-wives".to_string(), "https://www.literotica.com/c/mature-sex".to_string(), "https://www.literotica.com/c/mind-control".to_string(), "https://www.literotica.com/c/non-consent-stories".to_string(), "https://www.literotica.com/c/non-human-stories".to_string(), "https://www.literotica.com/c/erotic-novels".to_string(), "https://www.literotica.com/c/reviews-and-essays".to_string(), "https://www.literotica.com/c/adult-romance".to_string(), "https://www.literotica.com/c/science-fiction-fantasy".to_string(), "https://www.literotica.com/c/audio-sex-stories".to_string(), "https://www.literotica.com/c/masturbation-stories".to_string(), "https://www.literotica.com/c/transsexuals-crossdressers".to_string()];

    // We have a static list of categories we want to fetch. For every category we then figure out
    // how many pages are in that category's listing (e.g. https://www.literotica.com/c/anal-sex-stories/18-page, 19-page, etc.)
    println!("1) Fetching category pages: ");
    let cat_bar = ProgressBar::new(category_links.len() as u64);
    let cat_page_list =
    stream::iter(category_links)
        .map(|url| {
            let client = &literotica_client;
            async move {
                let burl = &url;
                match client.get(burl).send().await {
                    Ok(resp) => resp.bytes().await,
                    Err(e) => panic!("getting data failed")
                }
            }
        })
        .buffer_unordered(MAX_CONCURRENT_REQUESTS)
        .then(|resp| async {
            let bar = cat_bar.clone();
            match tokio::spawn(async move {
                let resp = match resp {
                    Ok(bytes) => bytes,
                    Err(e) => panic!("blah")
                };
                let response = std::str::from_utf8(&resp).unwrap();
                let parsed = Html::parse_document(response);

                let alpha_links_selector = Selector::parse(".b-alpha-links").unwrap();
                let link_selector = Selector::parse("a").unwrap();
    
                // equivalent to the following in the browser: 
                // document.querySelectorAll(".b-alpha-links")[0].querySelectorAll("a")[25].href
                // 25 here is because it's laid out by letter, so 25 is Z.
                let links_page = parsed.select(&alpha_links_selector).next().unwrap();
                let final_link_elem = links_page.select(&link_selector).last().unwrap();
                let final_link = final_link_elem.value().attr("href").unwrap(); // something like the following:
                // https://www.literotica.com/c/erotic-novels/95-page
                let slash_separated: Vec<&str> = final_link.split("/").collect();
                let category = slash_separated[4];
                let highest_page_num = slash_separated[5].split("-").next().unwrap().parse::<i32>().unwrap(); // the 95 from previous example
    
                let mut pages = vec![];
                for i in 1..highest_page_num+1 {
                    pages.push(CatInfo{
                        category: category.to_string(),
                        page: i,
                    })
                }
    
                bar.inc(1);
    
                stream::iter(pages)
            }).await {
                Ok(pages) => pages,
                Err(e) => panic!("got error with tokio spawn")
            }
        })
        .flatten_unordered(MAX_CONCURRENT_REQUESTS)
        .collect::<Vec<CatInfo>>().await;
    cat_bar.finish();



    // We have a list of all the category pages, which list stories, so we scrape every category page and get a list of all stories.
    println!("Found {} story listing pages", cat_page_list.len());
    let story_list_bar = ProgressBar::new(cat_page_list.len() as u64);
    println!("2) Fetching list of stories from category pages: ");
    let total_fetch_time = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let total_parse_time = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let story_list = stream::iter(&cat_page_list)
        .map(|page| {
            let client = literotica_client.clone();
            let category_url = format!("https://www.literotica.com/c/{}/{}-page", page.category, page.page.to_string());
            let fetch_clone = std::sync::Arc::clone(&total_fetch_time);
            let parse_clone = std::sync::Arc::clone(&total_parse_time);
            async {
                match tokio::spawn(async move {
                    let (parsed, fetch_time, parse_time) = fetch_and_parse(category_url, &client).await;
                    fetch_clone.fetch_add(fetch_time as u64, std::sync::atomic::Ordering::AcqRel);
                    parse_clone.fetch_add(parse_time as u64, std::sync::atomic::Ordering::AcqRel);
                    stream::iter(parsers::parse_story_listing(parsed))
                }).await {
                    Ok(resp) => {
                        story_list_bar.inc(1);
                        resp
                    },
                    Err(e) => panic!("tokio cat page spawn failed {}", e)
                }
            }
        })
        .buffer_unordered(20)
        .flatten_unordered(20)
        .collect::<Vec<String>>()
        .await;    
    story_list_bar.finish();

    let fetch_val = total_fetch_time.load(std::sync::atomic::Ordering::Acquire);
    let parse_val = total_parse_time.load(std::sync::atomic::Ordering::Acquire);

    let average_fetch_time: f64 = (fetch_val as f64)/(cat_page_list.len() as f64);
    let average_parse_time: f64 = (parse_val as f64)/(cat_page_list.len() as f64);
    println!("Fetch time is {}, parse time is {}", average_fetch_time/1_000_000.0, average_parse_time/1_000_000.0);

    // Scraping like this is somewhat unreliable and the current pipeline I'm using is very vulnerable to errors. Here we
    // implement a mechanism to scrape the site again, ignoring all the stories we've already scraped.
    let already_existing = match std::fs::File::open("/home/sophiawisdom/rust_scraping/results/metadata/little_bit.json") {
        Ok(file) => file,
        Err(e) => panic!("Got error while trying to open little_bit.json {}", e)
    };
    let already_grabbed_stories: Vec<StoryInfo> = serde_json::from_reader(already_existing).unwrap();
    let mut already_have_metadata = std::collections::HashSet::new();
    for story in &already_grabbed_stories {
        already_have_metadata.insert(story.storyName.clone());
    }
    let mut ungrabbed_stories = vec![];
    for story in &story_list {
        if !already_have_metadata.contains(story) {
            ungrabbed_stories.push(story);
        }
    }

    // let files = std::fs::read_dir("/home/sophiawisdom/rust_scraping/results/stories").unwrap();

    // Set up a fancy progress bar, because for a full scrape this can easily take 1-2h so we want good info on what's happening.
    println!("Fetching stories! There used to be {} but there are now {}", &story_list.len(), ungrabbed_stories.len());
    let story_parse_bar = ProgressBar::new(ungrabbed_stories.len() as u64);
    story_parse_bar.set_style(indicatif::ProgressStyle::default_bar()
    .template("parsing {msg}\n{pos}/{len} @ {per_sec} ETA: {elapsed_precise}/{duration_precise}\n{wide_bar}"));
    
    // We have a list of all the stories we want to scrape, and then we scrape them all and write them.
    let mut stories = already_grabbed_stories.clone();
    stream::iter(ungrabbed_stories)
    .map(|page| {
        let client = literotica_client.clone();
        let gage = page.clone();
        async {
            let mage = gage.clone();
            match tokio::spawn(async move {
                let page_url = format!("https://www.literotica.com/s/{gage}");
                let mut cur_url;
                let mut story_text = String::new();
                let first_page_info;
                {
                    let (first_page, _, _) = fetch_and_parse(page_url, &client).await;
                    first_page_info = parsers::parse_first_story_page(&first_page);
                    cur_url = match first_page_info.next_url.as_str() {
                        "" => None,
                        str => Some(format!("https://www.literotica.com/s/{}", str.to_string())),
                    };

                    let text_selector = Selector::parse("div.panel.article p").unwrap();
                    let mut stuff = vec![];
                    for el in first_page.select(&text_selector) {
                        stuff.push(el.text().collect::<Vec<_>>().join(""));
                    }
                    story_text.push_str(&stuff.join("\n"));
                }

                let next_page_button = Selector::parse("a[title=\"Next Page\"]").unwrap();
                let filename = format!("/home/sophiawisdom/rust_scraping/results/stories/{gage}.txt");
                loop {
                    cur_url = match cur_url {
                        Some(url) => {
                            let (fetched_page, _, _) = fetch_and_parse(url.to_string(), &client).await;

                            let text_selector = Selector::parse("div.panel.article p").unwrap();
                            let mut stuff = vec![];
                            for el in fetched_page.select(&text_selector) {
                                stuff.push(el.text().collect::<Vec<_>>().join(""));
                            }
                            story_text.push_str(&stuff.join("\n"));

                            match fetched_page.select(&next_page_button).next() {
                                Some(el) => {
                                    Some(format!("https://www.literotica.com/s/{}", el.value().attr("href").unwrap().to_string()))
                                },
                                None => None,
                            }
                        },
                        None => break
                    };
                }
                let mut story_file = std::fs::File::create(filename).unwrap();
                story_file.write_all(story_text.as_bytes()).unwrap();

                let time_since_epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
                stream::iter(vec![StoryInfo {
                    member_id: first_page_info.author_id,
                    member_name: first_page_info.author_name,

                    title: first_page_info.title,
                    description: first_page_info.description,
                    storyName: gage,
                    tags: first_page_info.tags,
                
                    favorites: first_page_info.favorites,
                    views: first_page_info.views,
                    rating: first_page_info.rating, // in centi-rating points, so e.g. 3.45 = 345

                    timeFetched: time_since_epoch.as_secs(),
                }])
            }).await {
                Ok(resp) => {
                    story_parse_bar.inc(1);
                    story_parse_bar.set_message(mage);
                    resp
                },
                Err(e) => {
                    eprintln!("failed with error {:?} for page {:?}", e, mage);
                    stream::iter(vec![])
                }
            }
        }
    })
    .buffer_unordered(MAX_CONCURRENT_REQUESTS)
    .flatten_unordered(MAX_CONCURRENT_REQUESTS)
    .map(|story| {
        stories.push(story);
        if stories.len() % 5000 == 0 {
            // When we do this we pause the whole application for gradually-increasing amounts of time as we get more stories.
            // This isn't ideal on a per-se efficiency basis, but it's important for us to double-scrape as few times as we can,
            // so it's worthwhile for us to do this.
            println!("writing {} files", &stories.len());
            let file = std::fs::File::create("/home/sophiawisdom/rust_scraping/results/metadata/little_bit.json").unwrap();
            serde_json::to_writer_pretty(file, &stories).unwrap();
        }
    }).collect::<Vec<_>>().await;

    println!("writing {} files", &stories.len());
    let file = std::fs::File::create("/home/sophiawisdom/rust_scraping/results/metadata/little_bit.json").unwrap();
    serde_json::to_writer_pretty(file, &stories).unwrap();
}
