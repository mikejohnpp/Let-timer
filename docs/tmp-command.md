let-timer list -> Show the schedule
-- priority <accended, decended> -> sort priority
-- priority-label "URGENT|IMMEDIATE|NOT YET" -> specific priority label can show
-- columns "name|time|..." -> specific column

let-timer create -> create entry
let-timer delete -> delete entries
let-timer find -> search entry
let-timer edit -> Edit one entry by id
let-timer current -> show current work need to done
let-timer start -> start the next work, if current work not done yet then continue this. If ctrl+c continue to run it on background, Show timer count stream.
let-timer stop -> stop current work.
let-timer done -> mark done the work.
let-timer restart -> restart daeamon
