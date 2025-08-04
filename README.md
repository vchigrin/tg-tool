# tg-tool - command-line tool for managing Telegram account

Currently it focuses on operations with dialog filters (Folders) in Telegram.

To accomplish this task program uses same API as ordinal Telegram clients, so you need to login to it in the same way as you do in ordinal Telegram client.

At present all commands require exlicit specification of `--session-file` parameter - this is path to the file where information required for communication with Telegram will be stored. Make sure this file is stored securely, do not share it with anybody. Person having it can do everything with you Telegram account (in particular, read and send messages).

## Supported commands

### login
Asks user for login credentials, perform login to Telegram servers and store session details in provided session file.

### logout
Terminates Telegram session, information about which is stored in session file. You can also terminate session through official Telegram client through Settings/Privacy and Security/Active sessions dialog.

### folders backup
Saves information about all Telegram folders in .json file. You can pass `--pretty` command-line flag if you want human-readable JSON.

### folders clear
Deletes all folders in Telegram. We recommend use "folders backup" command before using this.

### folders restore
Takes .json file, created by "folders backup" command, and attempts restore folder structure described in it. Don't removes dialogs from existing folders. If .json file specifies folder with the same name as existing, ensures that the same dialogs specified in .json file is present in current Telegram state.

### dialogs assign
Takes .json file with description of assignment rules, and assign dialogs to folders based on them. See information about rules for dialog assignment below.

## Rules for dialogs assignment
Rules file is file with JSON array of dicts, each specify rules, each specifying name for dialog filter and condition for assignment dialogs. Same dialog may be assigned to more then one folder. Note that these assignment rules are not supported by Telegram engine, so they will not be applied to new dialogs automatically. It is neccessary re-run this tool again to assign new dialogs.
Example:
```json
[
  {
    "name": "Robots",
    "condition": {
      "title_regex": {
        "regex_match": "(?i).*(robo)|(робо).*"
      }
    }
  }
]
```
Below description of rules and their attributes:

### title_regex
 Contains `regex_match` key, with string - regular expression, applyed to dialog name. Regex syntax is the same as used by Rust [regex](https://docs.rs/regex/latest/regex/) crate.

### info_regex
 Contains `regex_match` key, with string - regular expression, applyed to dialog "about" description of group chat or channel. Regex syntax is the same as used by Rust [regex](https://docs.rs/regex/latest/regex/) crate.

### contact_present
 Contains `login` field with login of bot/user (without `@` sign). If this contact is present among group participants, condition match.

### dialog_type
 Contains `dialot_type` field with one of strings: `User`, `Group`, `Channel` describing type of dialog to match.

### external_executable
 Uses external program to classify dialog. If it execution finishes with zero code, condition matches.
 Contains two keys:
 - `path` - path to executable file (with possible `~` expansion on `*nix` systems.
 - `params` - array of strings with command line arguments to pass during program invokation. Some elements may be "placeholder" strings, replaced by info of particular dialog.
     - `"@user_login@"` - replaced by login of the user, if this dialog with user. For other dialogs does not match.
     - `"@group_login@"` - replaced by login of the group, if this dialog with group which have one. For other dialogs does not match.
     - `"@channel_login@"` - replaced by login of the channel, if this dialog is subscription to channel with provided login. For other dialogs does not match.
     - `"@id@"` - replaced by numerical ID of the user/group/channel.

### not
 Value is another condition, parent matches when child does not match.
 Example:
 ```json
  "condition": {
    "not": {
      "info_regex": {
        "regex_match": "some_regex"
      }
    }
  }
 ```

### and, or
 These condition contain one key - `children` with array of JSON dicts, specifying child condition. Parent condition matches when either all conditions matches (`and`), or at least one matches (`or`).
 Example:
 ```json
  "condition": {
    "and": {
      "children": [
        {
          "contact_present": {
            "login": "some_login"
          }
        },
        {
          "info_regex": {
            "regex_match": "some_regex"
          }
        }
      ]
    }
  }
 ```
## not_matched
 Does not have any parameters. Matches if there are no rules in file before that, that match current dialog. That is why dialog of rules is important.
 Example (it matches all channels, not matches by rules above):
 ```json
  "condition": {
    "and": {
      "children": [
        {
          "dialog_type": {
            "dialog_type": "Channel"
          }
        },
        "not_matched"
      ]
    }
  }
 ```
