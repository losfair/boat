pub enum ServiceCursor<T> {
  Initial,
  Next(T),
  End,
}

impl<T> ServiceCursor<T> {
  pub fn as_request_cursor_ref(&self) -> Option<&T> {
    match self {
      ServiceCursor::Initial => None,
      ServiceCursor::Next(t) => Some(t),
      ServiceCursor::End => None,
    }
  }
}
