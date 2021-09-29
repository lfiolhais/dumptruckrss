# Dumptruckrss

Dumptruckrss is your go to shop to download stuff from an RSS feed. It features
queries to help you download just what you want. You can query for titles,
descriptions, dates and more.

Dumptruckrss is written using tokio and async, taking full advantage of your
system to maximize the number of concurrent downloads.

# Usage

To dump a whole feed downloading 5 items concurrently
```
dumptruckrss -u FEED download -d 5 -o FOLDER
```

To download the latest episode
```
dumptruckrss -u FEED -q latest download -o FOLDER
```

To download all episodes containing the word cheese in the description
```
dumptruckrss -u FEED -q description:cheese download -o FOLDER
```

To download all episodes containing the phrase 'cheese delight' in the title
```
dumptruckrss -u FEED -q 'title:cheese delight' download -o FOLDER
```
Check the help flag (`-h`) for more options on the description and title queries.

To download episodes 1 to 20.
```
dumptruckrss -u FEED -q number:[1-20] download -o FOLDER
```

If you are uncertain about a query and want to perform a dry run to check the results,
use the keyword `check` in the previous examples instead of `download`.

It is also possible to create a new feed based on a query. Using the previous
example, the following command creates a new feed, with the title Cheesy, with
all items that contain 'cheese delight' in the title to the file `my-feed.xml`.
```
dumptruckrss -u FEED -q 'title:cheese delight' create -o my-feed.xml -t Cheesy
```

# License

The code in this repository is licensed under GPLv3. For more information check the
[license](LICENSE) file.
