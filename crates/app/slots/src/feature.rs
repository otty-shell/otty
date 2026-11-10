use iced::Subscription;

/// What every feature gives the frame: its events, its answer and how it
/// changes.
///
/// The binary turns a click, a key, a tick or an I/O answer into the
/// feature's [`Feature::Intent`] and calls [`Feature::reduce`]. `reduce`
/// changes the feature's state and returns what happened as a list of
/// [`Feature::Outcome`]s. The binary reads each outcome in the same update:
/// news for another feature goes to that feature's `reduce` right away, I/O
/// starts in the binary's `effects/`, and a window operation (focus,
/// scrolling) becomes an iced task. So `reduce` does no I/O and sends no
/// message itself.
///
/// Nothing calls features through this trait generically: every call names
/// its feature. The trait writes the socket down, so the compiler checks
/// that each feature has the same shape.
///
/// # Example
///
/// ```
/// use otty_app_slots::Feature;
///
/// struct Bell {
///     unseen: usize,
/// }
///
/// enum BellIntent {
///     Rang,
///     Opened,
/// }
///
/// impl Feature for Bell {
///     type Intent = BellIntent;
///     type Outcome = ();
///     type Ctx<'a> = ();
///
///     fn reduce(&mut self, intent: BellIntent, _ctx: ()) -> Vec<()> {
///         match intent {
///             BellIntent::Rang => self.unseen += 1,
///             BellIntent::Opened => self.unseen = 0,
///         }
///
///         Vec::new()
///     }
/// }
///
/// let mut bell = Bell { unseen: 0 };
/// let _ = bell.reduce(BellIntent::Rang, ());
///
/// assert_eq!(bell.unseen, 1);
/// ```
pub trait Feature {
    /// The feature's input: "do this". Its view, its subscription, an I/O
    /// answer or the binary on behalf of another feature makes it.
    type Intent;

    /// What `reduce` reports: "this happened, this is what I need from
    /// outside". Only `reduce` makes it.
    type Outcome;

    /// Outside data the feature reads while it reduces, built by the binary
    /// from other features' getters at the moment of the decision; `()` when
    /// the feature reads none.
    type Ctx<'a>;

    /// Change the state for `intent` and return what happened, in order.
    fn reduce(
        &mut self,
        intent: Self::Intent,
        ctx: Self::Ctx<'_>,
    ) -> Vec<Self::Outcome>;

    /// The events the feature listens to on its own, such as a timer; none
    /// by default. The binary wraps them into its message.
    fn subscription(&self) -> Subscription<Self::Intent> {
        Subscription::none()
    }
}
