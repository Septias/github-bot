## DC-Github-Bot

A Deltachat-bot which works as bridge between Deltachat and Github Webhoooks

#### Client requests
Users can interact with the bot by issuin `commands`.
All `commands` have to be prefixed with `gh` and can be of the following form:

```rust
enum Cli {
    /// Subscribe to an event
    Subscribe {
        /// Id of the repository
        repo: usize,

        Pr {
            pr_action: PRAction,
        },
        Issue {
            #[arg(value_enum)]
            issue_action: IssueAction,
        },
    },

    /// Unsubscribe from an event
    Unsubscribe {
        /// Id of the repository
        repo: usize,

        Pr {
            #[arg(value_enum)]
            pr_action: PRAction,
        },

        Issue {
            #[arg(value_enum)]
            issue_action: IssueAction,
        },
    },

    // Change and list supported repositories
    Repositories {
        // List all available repositories
        List,

        // Add a webhook for a new repository
        Add {
            // Name of repo owner (user or organisation)
            owner: String,

            // Name of repository
            repository: String,

            // REST-Api key
            api_key: String,
        },

        // Remove a repositories webhook
        Remove {
            // Id of repository to remove
            repository: usize,

            // REST-Api key
            api_key: String,
        },
    },
}
```

#### Examples

Add a repository:
```
gh repositories add septias github-bot ghp_xyp
```
where `ghp_xyp` is a github rest-api key that can be created like [this](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token)

Add an event listener:
```
gh subscribe 123534 issue opened
```
where 123534 is a valid repo taken from:

List all repositories:a
```
gh repositories list
```

### Architecture
- The bot has to be hosted under a public ip to be able to receive rust webhooks.
- The file `server.rs` spins up a `tide` webserver listening on port `0.0.0.0:8080/receive`.
- The webhook sends all events to this endpoint where they are parsed and passed to the bot via an channel.
- The bot simultaneously listenes to client-requests over dc-api and for the webhooks.
- The client requests are parsed using `clap`.

#### Files
.
├── mock <- mock-files for testing
├── src
│ ├── bot.rs <- main bot code
│ ├── db.rs <- surrealdb-api
│ ├── main.rs <- spin up bot
│ ├── parser.rs <- creation of cli using `clap`
│ ├── queries <- some of the sql-queries used in `db.rs`
│ ├── rest_api.rs <- interaction with the github rest-api
│ ├── server.rs <- spin up `tide` server
│ ├── shared.rs <- some types
│ └── utils.rs

### Further improvement
- Don't allow users to register listeners twice
  - this gets rejected internally, but is not shown to user
