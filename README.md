## Rust Scraping
One of my first projects when I was learning to code was a scraper for literotica.com. When I was done, I posted a big archive of my scrape online and still get 3-5 requests a week for the file, so I thought I would redo this scrape but this time in Rust. The scraper is reasonably fast, fetching about 80 stories per second on my laptop or about 20MBps. Weirdly, just like my original Python parser, it is CPU-limited on HTML parsing. It's fast enough that it's not a huge concern of mine to optimize this bottleneck, though eventually I would like to figure out how to improve this. Mainly this is interesting because I don't understand why my reasonably specced-out laptop could be maxing out at 2 MB/s/core of HTML parsing.

This scraper respects no limits and is generally somewhat rude, so please try not to run it too many times. It is also not really written to run on other people's systems (e.g. hardcoded file links to my user directory).

Please be merciful when evaluating the code lol this was my first Rust project and I had a bunch of trouble figuring out how to get Async and Ownership to work together correctly.
