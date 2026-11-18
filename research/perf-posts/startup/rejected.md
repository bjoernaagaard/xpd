# Posts excluded from the timing table

- [Authon](https://blog.authon.dev/python-3-15-vs-3-14-vs-3-13-i-benchmarked-it-the-jit-barely-helped) names nine imports and says the CLI imports several others. The 76.9 and 21.4 ms comparison lacks the measured CLI, the complete import list and the early exit code.
- [Namegarage](https://namegarage.com/blog/python-3-15-lazy-imports) measured three real entry points. It shows changes to individual imports but does not publish the back office package needed to run any of those entry points.
- [CLI subcommands with lazy imports](https://snarky.ca/subcommands-with-lazy-imports/) supplies working examples without startup measurements.
- [Lazy imports using wrapt](https://grahamdumpleton.me/posts/2025/10/lazy-imports-using-wrapt/) shows how a proxy behaves without timing the import.
- [Python lazy imports you can use today](https://pythontest.com/python-lazy-imports-now/) inserts a one second sleep in the example module. The measured saving comes from skipping that sleep.
- [The Import Tax](https://arxiv.org/abs/2609.02753) is a research paper surveying package releases. It falls outside the request for blogs and posts.
- [Conda startup tracking](https://github.com/jezdez/conda-tempo/blob/main/track-a-startup.md) describes several changes and says the original six probe startup harness is private.
- [Real Python's lazy imports tutorial](https://realpython.com/python315-lazy-imports/) supplies a measured CLI and public code, but its article returned HTTP 403 on repeated requests from this machine. It missed the required URL check.
- [PEP 781 discussion, reply 122](https://discuss.python.org/t/85728/122) times a custom standard library against the original. Its change to typing is absent from the requested stock interpreters.
- [PEP 781 discussion, reply 175](https://discuss.python.org/t/85728/175) lists times from 3.7 through 3.14a6 without giving the command, clock or baseline used for the list.

## Search coverage

The Hacker News Algolia story search used the January 2025 cutoff with “python startup” and “python lazy imports”. It led to Posting, VEX, Namegarage and the sixteen import CLI. The Lobsters python and performance feeds mainly led to tool announcements. The Python discussion search for importtime returned the typing measurements. The PyCoder’s Weekly issues included the pypistats experiment among proposals, talks and tutorials.

The Python Weekly sitemap provided dated issue URLs for 2025 and 2026. All 77 dated issue requests returned HTTP 403. The Planet Python feed worked, but it is a rolling feed. Searches of the release schedules and talk lists at PyCon US and EuroPython in 2025 and 2026 found the lazy imports talk, without another qualifying article. The CPython issue search found issue 154675 tracking standard library import costs. Project tracking issues, speed.python.org and the public CPython benchmark repository supply interpreter context, but they do not give paired timings for these application paths.

The final passes through the conference programmes, CPython issue search and newsletter links yielded no additional qualifying post. The blocked archive limits coverage.
