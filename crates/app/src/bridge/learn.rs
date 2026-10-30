//! Courses, lessons, glossary, fretboard quiz and ear training (LH-LRN).

use super::{local_path, q, qobject};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

impl qobject::Controller {
    pub(super) fn learn_json(&self) -> QString {
        q(&self.rust().core.learn_json())
    }

    pub(super) fn lesson_json(&self, id: &QString) -> QString {
        q(&self.rust().core.lesson_json(&id.to_string()))
    }

    pub(super) fn lesson_seen(mut self: Pin<&mut Self>, id: &QString, page: i32) {
        let page = usize::try_from(page).unwrap_or(0);
        self.as_mut().core().lesson_seen(&id.to_string(), page);
    }

    pub(super) fn lesson_ear(
        mut self: Pin<&mut Self>,
        id: &QString,
        page: i32,
        block: i32,
    ) -> bool {
        let (page, block) = (usize::try_from(page), usize::try_from(block));
        let (Ok(page), Ok(block)) = (page, block) else {
            return false;
        };
        self.as_mut().set_playing(false);
        let state = self
            .as_mut()
            .core()
            .lesson_ear(&id.to_string(), page, block);
        let started = state.is_ok();
        self.as_mut().show_ear(state);
        started
    }

    pub(super) fn term_ear(mut self: Pin<&mut Self>, id: &QString, block: i32) {
        let Ok(block) = usize::try_from(block) else {
            return;
        };
        self.as_mut().set_playing(false);
        let state = self.as_mut().core().term_ear(&id.to_string(), block);
        self.as_mut().show_ear(state);
    }

    /// The ear state of a lesson or glossary block, or its error ("" for none).
    fn show_ear(mut self: Pin<&mut Self>, state: Result<String, String>) {
        match state {
            Ok(state) => self.as_mut().set_ear_state(q(&state)),
            Err(text) if text.is_empty() => {}
            Err(text) => self.as_mut().show_error(&text),
        }
    }

    pub(super) fn lesson_answer(
        mut self: Pin<&mut Self>,
        id: &QString,
        page: i32,
        block: i32,
        option: i32,
    ) -> bool {
        let at = (
            usize::try_from(page),
            usize::try_from(block),
            usize::try_from(option),
        );
        let (Ok(page), Ok(block), Ok(option)) = at else {
            return false;
        };
        self.as_mut()
            .core()
            .lesson_answer(&id.to_string(), page, block, option)
    }

    pub(super) fn review_json(&self) -> QString {
        q(&self.rust().core.review_json())
    }

    pub(super) fn chapter_check_json(&self, chapter: &QString) -> QString {
        q(&self.rust().core.chapter_check_json(&chapter.to_string()))
    }

    pub(super) fn chapter_pass(mut self: Pin<&mut Self>, chapter: &QString, since: f64) -> bool {
        // Seconds since 1970 from JSON; a negative or broken value asks for nothing earlier.
        let since = if since.is_finite() && since > 0.0 {
            since as u64
        } else {
            0
        };
        self.as_mut()
            .core()
            .chapter_pass(&chapter.to_string(), since)
    }

    pub(super) fn course_preview(&self, url: &QString) -> QString {
        let path = local_path(&url.to_string());
        q(&self.rust().core.course_preview(&path))
    }

    pub(super) fn course_add(mut self: Pin<&mut Self>, url: &QString) -> QString {
        let path = local_path(&url.to_string());
        q(&self.as_mut().core().course_add(&path))
    }

    pub(super) fn course_fetch(mut self: Pin<&mut Self>, url: &QString) {
        let json = self.as_mut().core().course_fetch(&url.to_string());
        self.as_mut().set_course_fetch_json(q(&json));
    }

    pub(super) fn lesson_video(&self, id: &QString, page: i32, block: i32) -> QString {
        let (Ok(page), Ok(block)) = (usize::try_from(page), usize::try_from(block)) else {
            return QString::default();
        };
        q(&self.rust().core.lesson_video(&id.to_string(), page, block))
    }

    pub(super) fn course_remove(mut self: Pin<&mut Self>, id: &QString) -> QString {
        q(&self.as_mut().core().course_remove(&id.to_string()))
    }

    pub(super) fn choose_course(mut self: Pin<&mut Self>, id: &QString) {
        self.as_mut().core().choose_course(&id.to_string());
    }

    pub(super) fn lesson_exercise(
        mut self: Pin<&mut Self>,
        id: &QString,
        page: i32,
        block: i32,
    ) -> bool {
        let (page, block) = (usize::try_from(page), usize::try_from(block));
        let (Ok(page), Ok(block)) = (page, block) else {
            return false;
        };
        let path = self
            .as_mut()
            .core()
            .lesson_exercise(&id.to_string(), page, block);
        self.as_mut().open_exercise(path)
    }

    pub(super) fn finale_exercise(mut self: Pin<&mut Self>, chapter: &QString) -> bool {
        let path = self.as_mut().core().finale_exercise(&chapter.to_string());
        self.as_mut().open_exercise(path)
    }

    /// Leaves ear training and opens a lesson's exercise like any file.
    fn open_exercise(mut self: Pin<&mut Self>, path: Result<std::path::PathBuf, String>) -> bool {
        match path {
            Ok(path) => {
                self.as_mut().ear_stop();
                let result = self.as_mut().core().open(&path);
                self.as_mut().after_open(result)
            }
            Err(text) => {
                if !text.is_empty() {
                    self.as_mut().show_error(&text);
                }
                false
            }
        }
    }

    pub(super) fn glossary_json(&self, id: &QString) -> QString {
        q(&self.rust().core.glossary_json(&id.to_string()))
    }

    pub(super) fn glossary_list_json(&self) -> QString {
        q(&self.rust().core.glossary_list_json())
    }

    pub(super) fn term_json(&self, topic: &QString) -> QString {
        q(&self.rust().core.term_json(&topic.to_string()))
    }

    pub(super) fn quiz_start(
        mut self: Pin<&mut Self>,
        kind: &QString,
        low: i32,
        high: i32,
    ) -> QString {
        q(&self
            .as_mut()
            .core()
            .quiz_start(&kind.to_string(), low, high))
    }

    pub(super) fn quiz_next(mut self: Pin<&mut Self>) -> QString {
        q(&self.as_mut().core().quiz_next())
    }

    pub(super) fn quiz_answer(mut self: Pin<&mut Self>, midi: f64) -> bool {
        self.as_mut().core().quiz_answer(midi)
    }

    pub(super) fn ear_start(mut self: Pin<&mut Self>, kind: &QString) {
        self.as_mut().set_playing(false);
        match self.as_mut().core().ear_start(&kind.to_string()) {
            Ok(state) => self.as_mut().set_ear_state(q(&state)),
            Err(text) => self.as_mut().show_error(&text),
        }
    }

    pub(super) fn ear_play(mut self: Pin<&mut Self>) {
        let state = self.as_mut().core().ear_play();
        self.as_mut().set_ear_state(q(&state));
    }

    pub(super) fn ear_next(mut self: Pin<&mut Self>) {
        let state = self.as_mut().core().ear_next();
        self.as_mut().set_ear_state(q(&state));
    }

    pub(super) fn ear_stop(mut self: Pin<&mut Self>) {
        self.as_mut().core().ear_stop();
        self.as_mut().set_ear_state(QString::default());
    }

    pub(super) fn ear_answer(mut self: Pin<&mut Self>, midi: i32) {
        let state = self.as_mut().core().ear_answer(f64::from(midi));
        self.as_mut().set_ear_state(q(&state));
    }

    pub(super) fn quiz_json(&self) -> QString {
        q(&self.rust().core.quiz_json())
    }
}
