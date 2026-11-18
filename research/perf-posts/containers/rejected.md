# Posts left out

| Post | Reason |
| --- | --- |
| [Tuples should remember their hash value](https://bugs.python.org/issue9685) | The 2013 experiment predates the supported interpreters. It is the historical comparison for issue 131525. |
| [Setdefault versus defaultdict](https://api.stackexchange.com/2.3/questions/38625608?site=stackoverflow) | The timed answers predate 2025 and do not measure Python 3.13. |
| [Using the Python defaultdict type](https://realpython.com/python-defaultdict/) | The grouping experiment predates 2025 and does not identify a supported interpreter. |
| [About defaultdict.__missing__](https://discuss.python.org/t/106922) | This is a concurrency discussion. It does not time the two grouping loops. |
| [Counter versus dict tally](https://www.shoppercove.com/blog/counter-vs-dict-tally-localhost-lab) | The store cites a workspace relative script path. It supplies neither that script nor enough input generation code to reconstruct the measured token stream. |
| [Hash collisions for tuples](https://bugs.python.org/issue34751) | The discussion predates the target versions and examines a different hash change. |
| [Knowing Python to write faster code](https://ipython-books.github.io/51-knowing-python-to-write-faster-code/) | These cookbook timings predate the target versions. |
| [Quadratic set and dictionary performance](https://pycoders.com/link/17060/web) | The workload deliberately forces hash collisions. Reproducing hash flooding falls outside this collection of ordinary container workloads. |
| [The fastest Python struct](https://www.crumpledpaper.tech/2026-06-21-python-struct-profiling/) | The comparison concerns representations of named records. |
| [Measuring the performance of CPython](https://us.pycon.org/2024/schedule/presentation/10/) | This talk describes how to measure the interpreter rather than comparing container code. |
| [Building a new tail calling interpreter](https://www.ep2025.europython.eu/session/building-a-new-tail-calling-interpreter-for-python/) | The benchmark measures interpreter dispatch. |
| [Reading the instance dictionary](https://deadlovelll.github.io/2026-09-05-reading-dict-deoptimizes-attribute-access/) | The operation being measured is attribute access. |
| [Python basic operations in 28 measurements](https://qiita.com/harupython/items/f635fb9a66bd39afe6a3) | This is a list of unrelated microbenchmarks. |
| [List versus tuple interview guide](https://impldev.com/en/interview/python/list-vs-tuple) | The guide repeats issue 131525 for MDP. It reports a separate hash timing, but the page does not identify the person who measured it or a publication date. |
| [Buffered merge follow up](https://discuss.python.org/t/93521/19) | The reported timings use CPython 3.12.4. Its merge2 function also raises NameError when the second iterator starts empty. |
| [Later merge timings](https://discuss.python.org/t/93521/20) | This follow up reuses the same data and partly unpublished revisions of the merge functions. Post 17 contains the author's complete original comparison. |

## Search coverage

The HN Algolia story search used the January 2025 cutoff with tuple hash, Python sets and dictionaries, and defaultdict. The Python and performance tag feeds on Lobsters supplied leads. Python Discuss search and the original merge threads supplied the posted code. The CPython issue search led to issue 131525, the old tracker discussion, the merge of PR 131529 and the separate measurements in PR 131922.

PyCoder's Weekly issue 718 linked the deque article. Issue 752 linked the hash collision article. The Python Weekly archive for 2025 and 2026, Planet Python and the session lists for PyCon US and EuroPython in both years were also checked. The final checks of the grouping literature, the Python Weekly archive and the Lobsters tag results added no new qualifying post. These were searches and samples of archives, not a count of every item published.

The two local rules give measurements on Python 3.15.2. The counting article supplies an independent comparison for counting-loop-without-counter. No qualifying independent comparison was found for setdefault-without-defaultdict. No rules were edited.

The local grouping check used 100,000 pairs with 50 integer keys. On 3.15.2, setdefault took 2.85 ms and defaultdict took 1.71 ms. Run `./run.sh group` to repeat it. The counting check with the same key count is the 100,000 integer row in the count files.
