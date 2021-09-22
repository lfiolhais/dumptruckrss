# Dumptruckrss

Dumptruckrss is your go to shop to download stuff from an RSS feed. It features
queries to help you download just what you want. You can query for titles,
descriptions, dates and more.

Dumptruckrss is written using tokio and async, taking full advantage of your
system to maximize the number of concurrent downloads.

# Usage

To dump a whole feed downloading 5 items concurrently
```
dumptruckrss -u FEED -o FOLDER -d 5 download
```

To download the latest episode
```
dumptruckrss -u FEED -o FOLDER -q latest download
```

To download all episodes containing the word cheese in the description
```
dumptruckrss -u FEED -o FOLDER -q description:cheese download
```

To download all episodes containing the phrase 'cheese delight' in the title
```
dumptruckrss -u FEED -o FOLDER -q 'title:cheese delight' download
```
Check the help flag (`-h`) for more options on the description and title queries.

To download episodes 1 to 20.
```
dumptruckrss -u FEED -o FOLDER -q number:[1-20] download
```

If you are uncertain about a query and want to perform a dry run to check the results,
use the keyword `check` in the previous examples instead of `download`.

# License

The code in this repository is licensed under GPLv3. For more information check the
[license](LICENSE) file.
