# What Artosia is made of

These are the things a volunteer meets in Artosia, described in plain English. A writer of any language reads this file and the briefs, then chooses **one word or short phrase in their own language for each thing before writing any string**, and writes that choice down in their word list. Every text that names a thing uses the chosen word, so one thing never gets two names. This file never names a thing in Indonesian or any other language, and nothing in it is a word to copy: it describes, and the writer picks the word that a church volunteer in that language already says.

The ids in backticks are the ones `concepts` lists in each brief.

## The service and its list

**`plan`** The ordered list of everything that will be put on the screen during one service: songs, Bible passages, the announcements. It lives in the left column of the main window. A church keeps one plan for each service of the week and switches between them. A plan has a name and, optionally, a date.

**`plan_item`** One entry of a plan: one song, one Bible passage, or the announcements as a whole. Clicking an item opens it in Preview; it does not show it to the room.

**`divider`** A named line inside a plan that splits it into parts, such as the part where the congregation sings and the part where scripture is read. It holds nothing to show and is never put on the screen. Churches usually call these parts of the service by short names.

**`song`** A song of the church: a title, an author, its words divided into parts, and the order the parts are sung in. All songs live in the library.

**`library`** Everything the church has stored: all the songs and all the Bible translations on this computer.

**`announcement`** and **`card`** The announcements are a set of notices, each notice a card with a title, some words and sometimes a picture. They are shown on the projector before the service begins, one after another in a loop. A card may also fill itself: with the words of a Bible passage, or with the names of people whose birthday falls this week or this month. The announcements appear in the plan as a single item.

**`passage`** A stretch of the Bible named by a book, a chapter and verses, such as the beginning of a well-known psalm. A passage is shown on the projector like a song, a few verses at a time.

**`bible_translation`** One version of the Bible in one language. The church can have several; one is used for showing passages and a second can be shown beside it. Translations are downloaded from a free list or brought in from a file the church already has. Each carries its own notice of who owns the rights to it.

**`song_order`** The sequence in which the parts of a song are sung. It can repeat a part: the repeated part is often sung after every numbered part. It is shown as a row of small chips in the song editor.

**`second_language`** A song may carry its words in two languages; the second language shows under the first on the screen.

**`licence`** Permission to show song words in church. Churches usually hold one from a licensing body whose name is the letters CCLI, and write its number in Settings. CCLI is a name and stays as it is.

## What a song is made of

**`section`** One named part of a song: the numbered parts, the repeated part, the lead-in, the bridge, the opening, the closing. A song is a row of sections, and each section becomes one or more slides.

**`verse`** The numbered parts of a song. Each is sung once, with the same tune, so a song has verse 1, verse 2 and so on. Not the same as a verse of the Bible, which is also called a verse in English; a writer must choose a word that cannot be mistaken for a Bible verse when the context is a song.

**`chorus`** The part that comes back after the numbered parts, with the same words each time. It is the part a whole congregation knows by heart.

**`pre_chorus`** A short lead-in sung just before the chorus, leading up to it.

**`bridge`** A part sung once, later in the song, in a different tune from the others, usually just before the last chorus.

**`intro`** The opening of a song before the first part, played or sung.

**`ending`** The closing part of a song, such as a final line sung slowly.

**`other_part`** Any part of a song that is none of the kinds above. It is numbered.

**`slide`** One screenful of words on the projector. A song is cut into slides, usually two to six lines each, and a Bible passage into slides of a few verses each. The volunteer moves from slide to slide.

**`tidy`** An editor button that cleans up words pasted from elsewhere: lines that are only the name of a part become program markers, and lines that are only guitar chords are removed.

## The three panes and how things reach the room

**`preview`** The middle pane of the main window: the slides of what will be shown next, laid out as a list. Nothing in it is ever seen by the congregation. A volunteer opens things here to check them first.

**`ring`** A white outline drawn round one slide in the Preview list. It marks the slide that Show will send next. It moves with the up and down arrow keys while Ctrl is held. It is a pointer on a slide, not a border for decoration and not a part of the projector's picture.

**`live`** The right pane of the main window: the slides of what the congregation is seeing right now, with the red marker on the slide that is on the projector. Red always means live.

**`show`** The action of moving what is in Preview into Live, so that it becomes the thing the room sees. It is the big tile under the Preview list and the Enter key. It is the main act of running a service.

**`live_switch`** The tile named Live in the lower row under the Live list (key F6). It switches the projector's output on and off. When it is off, the Live list may still be filled but the room sees nothing of it. When it is on the tile is red. It is switched on at every start.

**`next_prev`** The two big tiles under the Live list. They move the projector one slide forward or one slide back, inside the live song and onwards into the next plan item. Next is also the Space key.

**`clear_text`** A state of the projector in which the words are hidden but the background picture or video stays on the screen (key F1, the tile that reads No text in English). It is lit while on.

**`black`** A state of the projector in which the whole screen is black (key F2). It is lit while on.

**`logo`** A state of the projector in which the church's emblem is shown by itself in place of the slide (key F3). The picture is chosen in Design. The tile is greyed out when none is chosen.

**`start_mark`** A mark on a song in the plan that names the section where the song will begin when its turn comes, for example its chorus. It exists for medleys, where the band goes from one song straight into the middle of the next. With no mark a song begins at its first slide. It is set from Preview with a small play button, or from the song's row menu in the plan, and the plan row then shows that section's chip.

**`earlier`** A small menu in the title of the Live pane that lists what was already shown during this service, so that a volunteer can bring a song back.

## The screens

**`projector`** The big screen the congregation reads: a second monitor or a projector, driven by Artosia from the same computer. The word covers both. The program calls the thing the congregation faces by this one noun everywhere.

**`stage_screen`** A second picture for the musicians and singers, facing them: the live words, the next slide, a clock, a timer and notices for the band. It can go to its own monitor or into a window.

**`screens_dialog`** The dialog where each monitor connected to the computer is given a job: the congregation's, the band's, or none. It also shows numbers on the monitors so they can be told apart.

**`wall_picture`** The physical picture as it falls on a wall when a projector sits at an angle. The dialog lets a volunteer pull the four corners until it looks square and adjust brightness, contrast and colour.

**`room_message`** A short notice typed by the volunteer and drawn over the slide on the congregation's screen, such as that a car's lights are on. A yellow pill in the Live title strip shows while one is up.

**`stage_message`** The same kind of notice, shown only on the musicians' screen.

**`countdown`** A timer counting down to the start of the service, drawn on the projector for the room while people take their seats. It is started by typing a number of minutes in the search or from the phone.

## Look

**`design`** The set of choices about how the projector looks: the picture or video behind the words, the colour, size, font, outline and position of the words, and the church's emblem. Also the name of the dialog and of its button. A plan can carry its own design.

**`backdrop`** What lies behind the words on the projector: a plain colour scheme, a picture or a video from the computer, or a live camera.

**`outline`** A thin line drawn round each letter of the words, so that they stay readable over a busy picture. It has a thickness (off, thin, middle, thick) and a colour. It is not the white ring of Preview.

**`settings`** The program's own preferences, apart from how the projector looks: the language of the window, light or dark window, Bible choices, the licence number, the search's memory, key behaviour, the version and the tour.

**`window_language`** The language the program's own words are shown in. Not the language sung in the room, which is a separate choice in the listener.

## Finding things

**`search`** The one box that finds a song, a Bible passage or an action. It opens as a panel over the window, from the top bar or by typing any letter. Results show a preview on the right and offer Open, Show now and Add to plan.

**`search_memory`** What the search learns from use on this computer: which song was picked for what was typed, which song follows which, which songs are sung on a given weekday, which section a song usually starts at. It only orders results that matched equally well. A switch turns it on and a button forgets it. It is not the library and forgetting it never touches a song.

**`suggestion`** An offer, in a strip at the top of Preview, of what to show next: because the listener heard it, because it is often sung after the live song, or because the singing seems to have left the words on the screen. A suggestion only ever fills Preview and is never sent to the room by itself.

**`listener`** The microphone helper. It listens to the room, works out which song or verse is being sung or read, and makes suggestions. It never shows anything on its own.

**`keyboard`** The shortcut keys of the window, listed in a sheet. Key names such as Ctrl, Shift, Enter and the F keys are drawn as printed on a keyboard and are never translated.

## Outside the window

**`phone_remote`** A web page opened on a volunteer's phone over the church's wireless network, which moves the slides, picks plan items, searches, sends notices and starts a countdown. It joins by a number shown on the computer. It has its own few words, listed in `brief-phone.json`.

**`stream`** Broadcasting the projector's picture, with the room's sound, live to a video website, and optionally keeping a file of it. It needs an address and a secret code from the video website.

**`import`** Bringing songs, Bibles, notices or birthdays in from outside: the older program EasiSlides, a folder of song files, a presentation file, a table of birthdays, a Bible file.

**`update`** The program checks by itself at every start whether a newer version exists and can be asked to check.

## Other

**`tour`** A five-stop walk-through of the window, shown at the first start and available again later. Each stop is a speech bubble pointing at a part of the window: the plan, Preview, Show, Live and the search.

**`practice`** Practice mode: the program runs with a made-up library, saves nothing, and keeps the projector off. For learning without risk.

**`undo`** Taking back the last removal: a plan item, a song, an announcement, a whole plan, the editor that was just closed, or what the search had learned. Offered in the toast that reports the removal.

**`notice`** A toast: a short message that appears near the bottom of the window for a few seconds after something happened and then goes. Not to be confused with an announcement card or a notice to the room.
