# Jajan privacy and service improvement notice

Version 2026-11-06.3

## Operator and contact

The operator is egoing. Contact egoing+support@gmail.com for privacy matters. GitHub Issues is available for general project questions. Do not post personal information publicly.

This notice describes optional diagnostic sharing for fixing errors, improving performance and investigating freezes. Sharing is off by default. Transmission begins only after you review this notice and consent separately. A change to the notice does not automatically carry forward prior consent.

## 1. If you decline

No errors, performance traces or spans, structured diagnostic logs or freeze-monitoring data are transmitted externally. A build containing a DSN or a development environment variable cannot substitute for consent. All features, including notes and terminals, remain available.

Notes, captures, terminal settings and consent choices needed for the app are stored locally. The diagnostic setting does not control network communication by external commands or tools you run.

## 2. Purpose and categories

With separate consent, the following data is collected and used to fix errors, improve performance and investigate freezes, and transferred to Sentry for processing and storage.

- App version, build identifier, operating system and CPU architecture.
- Sanitized error categories, approved app function and file names, and line numbers.
- Fixed operation names, durations, counts and failure counts, freeze and recovery types and times, and structured diagnostic logs.
- Trace identifiers linking requests and operations. No persistent user or device identifiers are created.

Diagnostics exclude note contents and titles, terminal input, output and commands, clipboard contents, screens and images, usernames, device names and local paths. Error messages, source context and absolute paths are removed. Complete anonymity cannot be guaranteed. The receiving service sees the connection IP address and may process or log it according to its server settings, even when the client removes IP fields.

Release Health sessions and session statistics are not collected pending confirmation of retention periods. Session Replay, attachments, screen recordings, memory dumps and per-function CPU profiles are not transmitted.

## 3. International transfer and processor

- Recipient: Functional Software, Inc. (Sentry).
- Recipient privacy contact: https://sentry.io/contact/gdpr/ . App operator: egoing+support@gmail.com.
- Country and storage location: Iowa, United States (US region).
- Timing: when diagnostic events occur or are aggregated during app use after consent.
- Method: encrypted HTTPS.
- Categories: the diagnostics in section 2 and the IP address processed during connection.
- Purpose: processing error, performance and freeze diagnostics and providing analysis to the operator.
- Retention and use: see the separate criteria for originals, backups and cumulative statistics in section 4.
- To decline, leave sharing unchecked initially or turn it off in Settings. All features remain available.

## 4. Retention of originals, backups and statistics

These periods reflect the Sentry Developer account checked on 2026-11-06 and the approved policy.

| Data | Retention or deletion |
| --- | --- |
| Original error events | Up to 30 days from collection |
| Original performance traces, spans and transactions | Up to 30 days from collection |
| Original structured diagnostic logs | Up to 30 days from collection |
| Sentry disaster recovery backups | Deleted 30 or 90 days after backup creation, depending on the data type |
| Cumulative error issue statistics | Counts may remain while the issue exists, after individual events expire. Complete deletion in 30 days is not promised. Contact the operator to request deletion. |
| Release Health sessions and statistics | Not collected pending confirmation of retention periods |

Backups are server recovery copies of transmitted diagnostic data. They do not back up your computer or notes. Backup retention starts at backup creation, separately from the 30-day retention of originals. These periods do not mean that all related data is completely deleted within 30 days of collection.

At the time of the account review, traceMetric aggregation was unused. The app sends no separate metric envelopes. Before adding long-term aggregate metrics, the operator will verify processing and retention conditions and update the notice and consent.

References: [Account review](../verification/sentry-data-retention-20261106.md), [Sentry retention](https://docs.sentry.io/security-legal-pii/security/data-retention-periods/), [backup deletion](https://www.sentry.help/en/articles/13965019-how-frequently-is-data-backed-up), [issue counts after events expire](https://www.sentry.help/en/articles/13964336-why-does-the-event-count-in-my-issue-not-match-up-with-the-count-in-the-all-events-tab-of-the-issue).

## 5. Choices, withdrawal and rights

Diagnostic sharing is unchecked by default and separate from agreement to the Terms. Selecting it consents to the collection, use and international transfer to Sentry described in sections 2–4. Withdraw at any time in Settings. Enabling sharing requires an explicit save. Disabling it takes effect immediately.

Withdrawal blocks new transmissions and prevents pending data from a previous consent period from being sent. Network requests already underway may finish. The withdrawal confirmation waits for them to finish. Deleting previously received data is separate from stopping future transmissions.

Send requests for access, correction or deletion to egoing+support@gmail.com. Since no user or device identifiers are stored, locating individual records may be difficult. Provide only the information necessary to verify your request. Do not post personal information or raw diagnostics publicly.

Consent, document version and decision time are stored locally without account or advertising identifiers. Transmission is blocked if saving fails, the file is damaged or document versions differ. Changing a setting does not automatically delete existing local data or data already received by the server.
