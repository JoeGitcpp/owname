#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationDomainKind {
    Axis,
    SlotFamily,
    RelationEdge,
    ReasoningFragment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationDomainRef {
    pub kind: ObservationDomainKind,
    pub primary: u16,
    pub secondary: u16,
    pub tertiary: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HorizonKind {
    Point,
    Window,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PredictionHorizon {
    pub kind: HorizonKind,
    pub publish_t_us: u64,
    pub target_start_t_us: u64,
    pub target_end_t_us: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrAlignmentKind {
    PointExact,
    WindowInclusive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationStatus {
    Open,
    Resolved,
    Expired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PredictionPublicationTrace {
    pub predictor_family: u8,
    pub domain: ObservationDomainRef,
    pub horizon: PredictionHorizon,
    pub alignment: ErrAlignmentKind,
    pub status: PublicationStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenPredictionTrace {
    pub predictor_family: u8,
    pub domain: ObservationDomainRef,
    pub horizon: PredictionHorizon,
    pub alignment: ErrAlignmentKind,
    pub publication_index: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VpcContractState {
    pub publications: Vec<PredictionPublicationTrace>,
    pub open_predictions: Vec<OpenPredictionTrace>,
}

impl VpcContractState {
    pub fn push_publication(&mut self, trace: PredictionPublicationTrace) {
        self.publications.push(trace);
    }

    pub fn push_open_prediction(&mut self, trace: OpenPredictionTrace) {
        self.open_predictions.push(trace);
    }
}
