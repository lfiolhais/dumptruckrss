# Dumptruckrss

Dumptruckrss is your go to shop to download stuff from an RSS feed. It features
queries to help you download just what you want. You can query for titles,
descriptions, dates and more.

Dumptruckrss is written using tokio and async, taking full advantage of your
system to maximize the number of concurrent downloads.

```
USAGE:
    dumptruckrss [OPTIONS] --output <OUTPUT> <--url <URL>|--file <FILE>> [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -f, --file <FILE>                RSS feed File
    -d, --ndownloads <NDOWNLOADS>    Maximum number of concurrent downloads
    -o, --output <OUTPUT>            Output location to download contents
    -q, --query <QUERY>              Query items with the following patterns:
                                     [date | title | description | number | notexists | latest]

                                     Examples:
                                        Number: Select items in the feed with the following numbers.
                                                'number:[0:12]' (range), 'number:20' (scalar), 'number:{[0:12], 20}' (set)
                                        Title: Select items which contain the keyword
                                                'title:my_title' (value) or 'title:{my_title, other_title}' (set)
                                        Description: Select items where their description contain the keyword(s)
                                                'description:my_word' (value) or 'description:{my_word, other_word}' (set)
                                        Not Exists: Select items which are not present in the specified directory
                                                'notexists'
                                        Latest: Select the latest item in the feed
                                                'latest' downloads the most recent item or 'latest:N' to download the N most recent
                                     items
    -u, --url <URL>                  RSS feed url

SUBCOMMANDS:
    check       Check query results
    download    Download queried items in this feed to the specified folder
    help        Prints this message or the help of the given subcommand(s)
```
