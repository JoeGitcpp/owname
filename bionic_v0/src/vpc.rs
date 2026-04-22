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
    pub fn record_open_publication(
        &mut self,
        publication: PredictionPublicationTrace,
        predictor_family: u8,
        domain: ObservationDomainRef,
        horizon: PredictionHorizon,
        alignment: ErrAlignmentKind,
    ) -> u32 {
        let idx = self.publications.len() as u32;
        self.publications.push(publication);
        self.open_predictions.push(OpenPredictionTrace {
            predictor_family,
            domain,
            horizon,
            alignment,
            publication_index: idx,
        });
        idx
    }

    pub fn resolve_open_publication(
        &mut self,
        predictor_family: u8,
        domain: ObservationDomainRef,
        target_start_t_us: u64,
        target_end_t_us: u64,
    ) -> bool {
        let Some(open_idx) = self.open_predictions.iter().position(|trace| {
            trace.predictor_family == predictor_family
                && trace.domain == domain
                && trace.horizon.target_start_t_us == target_start_t_us
                && trace.horizon.target_end_t_us == target_end_t_us
        }) else {
            return false;
        };

        let open = self.open_predictions.remove(open_idx);
        if let Some(publication) = self.publications.get_mut(open.publication_index as usize) {
            publication.status = PublicationStatus::Resolved;
        }
        true
    }
}
