use scraper::{Html, Selector};

#[derive(Default)]
pub struct FirstPageInfo {
    pub author_name: String,
    pub author_id: u32,

    pub title: String,
    pub description: String,
    pub tags: Vec<String>,

    pub favorites: u32,
    pub views: u32,
    pub rating: u32,

    pub next_url: String,
}

pub fn parse_story_listing(document: Html) -> Vec<String> {
    let story_link_selector = Selector::parse(".r-34i").unwrap();
    let mut pages = vec![];
    for story in document.select(&story_link_selector) {
        let page = story.value().attr("href").unwrap();
        if page.contains("stories/showstory.php") {
            continue;
        }
        let stripped = std::str::from_utf8(&page.as_bytes()[29..]).unwrap().to_string();
        pages.push(stripped)
    }
    pages
}

pub fn parse_first_story_page(document: &Html) -> FirstPageInfo {
    let mut info = FirstPageInfo::default();
    let infopage_selector = Selector::parse("div[role=tabpanel]").unwrap();
    let descriptor_selector = Selector::parse(".aK_B").unwrap();
    let tag_selector = Selector::parse(".av_as.av_r").unwrap();
    let title_selector = Selector::parse("h1.headline").unwrap();
    let author_selector = Selector::parse("a.y_eU").unwrap();
    let favorites_selector = Selector::parse("div[title=Favorites] span").unwrap();
    let views_selector = Selector::parse("div[title=Views] span").unwrap();
    let ratings_selector = Selector::parse("div[title=Rating] span").unwrap();
    let next_page_button = Selector::parse("a[title=\"Next Page\"]").unwrap();

    info.title = match document.select(&title_selector).next() {
        Some(el) => el.text().collect::<Vec<_>>().join(""),
        None => {
            eprintln!("Unable to find title for document!");
            "".to_string()
        }
    };
    
    let authorElem = document.select(&author_selector).next().unwrap();
    info.author_name = authorElem.text().collect::<Vec<_>>().join("");
    let mut spliterator = authorElem.value().attr("href").unwrap().split(&['=', '&']);
    spliterator.next();
    info.author_id = spliterator.next().unwrap().parse().unwrap();

    let mut infopages = document.select(&infopage_selector);
    let user_infobox = infopages.next().unwrap();
    info.description = user_infobox.select(&descriptor_selector).next().unwrap().text().collect::<Vec<_>>().join("");

    info.rating = match user_infobox.select(&ratings_selector).next() {
        Some(el) => {
            let element_text = el.text().collect::<Vec<_>>().join("");
            // Something like "4.59" or "4.50"
            let mut rating_string = element_text.replace(".", "");
            if rating_string.chars().count() < 3 {
                rating_string.push('0');
            }
            match rating_string.parse() {
                Ok(num) => num,
                Err(e) => {
                    eprintln!("Got error {:?} while parsing rating string {:?} {:?}", e, rating_string, info.title);
                    0
                }
            }
        },
        None => 0
    };

    info.views = match user_infobox.select(&views_selector).next() {
        Some(el) => {
            // something like "67.4k" indicating # of views
            let mut element_text = el.text().collect::<Vec<_>>().join("");
            let multiplier = match element_text.chars().last().unwrap() {
                'k' => {
                    element_text.pop();
                    1_000.0
                },
                'm' => {
                    element_text.pop();
                    1_000_000.0
                },
                _ => 1.0,
            };
            let num = match element_text.parse::<f64>() {
                Ok(num) => num,
                Err(e) => {
                    eprintln!("Got error {:?} while parsing string {:?}", e, element_text);
                    0.0
                }
            };
            (num*multiplier) as u32
        },
        None => 0,
    };

    info.favorites = match user_infobox.select(&favorites_selector).next() {
        Some(el) => match el.text().collect::<Vec<_>>().join("").parse() {
            Ok(num) => num,
            Err(_) => 0
        },
        None => 0
    };

    let tag_infobox = infopages.next().unwrap();
    let mut tags = vec![];
    for tag_elem in tag_infobox.select(&tag_selector) {
        tags.push(tag_elem.text().collect::<Vec<_>>().join(""));
    }
    info.tags = tags;

    info.next_url = match document.select(&next_page_button).next() {
        Some(el) => el.value().attr("href").unwrap().to_string(),
        None => "".to_string(),
    };

    // Not going to record series info -- hopefully that can be found on the member page.

    info
}

pub fn parse_story_text(document: &Html) -> String {
    let text_selector = Selector::parse("div.panel.article p").unwrap();
    "hello".to_string()
}
