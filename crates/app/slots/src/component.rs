use iced::Subscription;

/// How a part of the app changes its state and what it listens to.
///
/// A component is a part of the app with its own state: a feature, such as
/// the tabs or the containers, or a frame widget, such as the title bar or
/// the sidebar. A [`Component::Intent`] comes in, [`Component::reduce`]
/// changes the state, and a list of [`Component::Outcome`]s comes out.
/// [`Component::subscription`] gives the events the component listens to on
/// its own, such as a timer.
///
/// The trait has no view. A component draws through the place traits
/// ([`LeftPanel`](crate::LeftPanel), [`MainArea`](crate::MainArea),
/// [`RightPanel`](crate::RightPanel), [`StatusBar`](crate::StatusBar),
/// [`Overlays`](crate::Overlays)), one for each place it fills, or through
/// methods of its own, because one component can fill several places and a
/// frame view can draw data of several components.
///
/// The binary turns a click, a key, a tick or an I/O answer into the
/// component's intent and calls `reduce`. The binary reads each outcome in
/// the same update: news for another component goes to that component's
/// `reduce` right away, I/O starts in the binary, and a window operation
/// (focus, scrolling) becomes an iced task. So `reduce` does no I/O and sends
/// no message itself.
///
/// Nothing calls components through this trait generically: every call names
/// its component. The trait writes the socket down, so the compiler checks
/// that each component has the same shape.
///
/// # Example
///
/// ```
/// use otty_app_slots::Component;
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
/// impl Component for Bell {
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
pub trait Component {
    /// The component's input: "do this". Its view, its subscription, an I/O
    /// answer or the binary on behalf of another component makes it.
    type Intent;

    /// What `reduce` reports: "this happened, this is what I need from
    /// outside". Only `reduce` makes it.
    type Outcome;

    /// Outside data the component reads while it reduces, built by the
    /// binary from other components' getters at the moment of the decision;
    /// `()` when the component reads none.
    type Ctx<'a>;

    /// Change the state for `intent` and return what happened, in order.
    fn reduce(
        &mut self,
        intent: Self::Intent,
        ctx: Self::Ctx<'_>,
    ) -> Vec<Self::Outcome>;

    /// The events the component listens to on its own, such as a timer; none
    /// by default. The binary wraps them into its message.
    fn subscription(&self) -> Subscription<Self::Intent> {
        Subscription::none()
    }
}
