#![forbid(unsafe_code)]

use std::{collections::BTreeMap, env, time::Duration};

use mpd24_ai_groove_engine::{CandidateMetrics, GrooveCandidate, GrooveOperation, GrooveParams};
use mpd24_ai_sequencer::Pattern;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

pub const DEFAULT_BASE_URL: &str = "https://api.typesafe.ai";
pub const DEFAULT_MODEL: &str = "jev-latest";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JevStatus {
    pub configured: bool,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct JevConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout: Duration,
}

impl JevConfig {
    pub fn from_env() -> Option<Self> {
        let api_key = env::var("TYPESAFE_API_KEY")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())?;
        let base_url = env::var("TYPESAFE_BASE_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
        let model = env::var("TYPESAFE_DEFAULT_MODEL")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_MODEL.to_owned());
        Some(Self {
            api_key,
            base_url,
            model,
            timeout: Duration::from_secs(10),
        })
    }
}

#[derive(Debug, Clone)]
pub struct JevClient {
    config: JevConfig,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JevRerankDecision {
    pub selected_id: String,
    pub confidence: f32,
    pub probabilities: BTreeMap<String, f32>,
    pub safe_to_preview: f32,
    pub model: String,
    pub input_tokens: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PerformanceDirection {
    Hold,
    Build,
    Release,
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DirectorIntensity {
    Subtle,
    Balanced,
    Bold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JevDirectorDecision {
    pub operation: GrooveOperation,
    pub direction: PerformanceDirection,
    pub intensity: DirectorIntensity,
    pub confidence: f32,
    pub worth_changing: f32,
    pub model: String,
    pub input_tokens: u64,
}

#[derive(Debug, Error)]
pub enum JevError {
    #[error("failed to build Jev HTTP client: {0}")]
    Client(String),
    #[error("Jev request failed: {0}")]
    Transport(String),
    #[error("Jev returned HTTP {status}: {body}")]
    Http { status: StatusCode, body: String },
    #[error("Jev response is missing answer: {0}")]
    MissingAnswer(String),
    #[error("Jev selected an unknown candidate: {0}")]
    UnknownCandidate(String),
    #[error("Jev returned an invalid director choice: {0}")]
    InvalidDirectorChoice(String),
}

impl JevClient {
    pub fn new(config: JevConfig) -> Result<Self, JevError> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|error| JevError::Client(error.to_string()))?;
        Ok(Self { config, http })
    }

    pub fn status_from_env() -> JevStatus {
        match JevConfig::from_env() {
            Some(config) => JevStatus {
                configured: true,
                model: config.model,
            },
            None => JevStatus {
                configured: false,
                model: env::var("TYPESAFE_DEFAULT_MODEL")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| DEFAULT_MODEL.to_owned()),
            },
        }
    }

    pub async fn rerank(
        &self,
        source: &Pattern,
        candidates: &[GrooveCandidate],
        operation: GrooveOperation,
        params: &GrooveParams,
    ) -> Result<JevRerankDecision, JevError> {
        let request = build_request(&self.config.model, source, candidates, operation, params);
        let parsed = self.execute(request).await?;
        parse_decision(parsed, candidates)
    }

    pub async fn direct(
        &self,
        source: &Pattern,
        params: &GrooveParams,
        preference_memory: Value,
    ) -> Result<JevDirectorDecision, JevError> {
        let request = build_director_request(&self.config.model, source, params, preference_memory);
        let parsed = self.execute(request).await?;
        parse_director_decision(parsed)
    }

    async fn execute(&self, request: SystemOneRequest) -> Result<SystemOneResponse, JevError> {
        let response = self
            .http
            .post(format!("{}/v1/systemone", self.config.base_url))
            .bearer_auth(&self.config.api_key)
            .header("Accept", "application/json")
            .header("X-MPD24-AI-Integration", "jev-reranker/1")
            .json(&request)
            .send()
            .await
            .map_err(|error| JevError::Transport(error.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| JevError::Transport(error.to_string()))?;
        if !status.is_success() {
            return Err(JevError::Http { status, body });
        }
        let parsed: SystemOneResponse =
            serde_json::from_str(&body).map_err(|error| JevError::Transport(error.to_string()))?;
        Ok(parsed)
    }
}

#[derive(Debug, Serialize)]
struct SystemOneRequest {
    model: String,
    state: Value,
    questions: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct SystemOneResponse {
    model: String,
    answers: BTreeMap<String, Value>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u64,
    #[allow(dead_code)]
    output_tokens: u64,
}

fn build_request(
    model: &str,
    source: &Pattern,
    candidates: &[GrooveCandidate],
    operation: GrooveOperation,
    params: &GrooveParams,
) -> SystemOneRequest {
    let criteria = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.id.clone(),
                json!({
                    "seed": candidate.seed,
                    "metrics": candidate.metrics,
                    "guidance": candidate_guidance(&candidate.metrics, operation)
                }),
            )
        })
        .collect::<serde_json::Map<String, Value>>();

    let state = json!({
        "task": "Rerank symbolic drum-groove candidates for a performance instrument. Prefer musical coherence, rhythmic identity, useful variation, and preservation of user intent. Do not reward novelty for its own sake.",
        "operation": operation,
        "source": pattern_summary(source),
        "controls": {
            "amount": params.amount,
            "density": params.density,
            "complexity": params.complexity,
            "humanize": params.humanize,
            "lockedTracks": params.locked_tracks,
        },
        "candidates": candidates.iter().map(|candidate| json!({
            "id": candidate.id,
            "seed": candidate.seed,
            "metrics": candidate.metrics,
            "summary": compact_pattern_summary(&candidate.pattern),
        })).collect::<Vec<_>>()
    });

    let mut questions = BTreeMap::new();
    questions.insert(
        "bestCandidate".into(),
        json!({
            "type": "choice",
            "instructions": "Which candidate best satisfies the requested groove operation while remaining musically coherent and performance-ready?",
            "criteria": criteria,
        }),
    );
    questions.insert(
        "safeToPreview".into(),
        json!({
            "type": "noul",
            "instructions": "Does this candidate set contain at least one musically coherent option worth presenting to the user as a non-destructive preview?",
            "criteria": {
                "true": "At least one candidate is coherent, useful, and aligned with the requested operation.",
                "false": "The candidates are too weak, incoherent, or misaligned to recommend."
            }
        }),
    );

    SystemOneRequest {
        model: model.to_owned(),
        state,
        questions,
    }
}

fn build_director_request(
    model: &str,
    source: &Pattern,
    params: &GrooveParams,
    preference_memory: Value,
) -> SystemOneRequest {
    let state = json!({
        "task": "Act as a performance director for a symbolic drum machine. Decide what musical transformation should be attempted next. Preserve groove identity, locked tracks, and performance usefulness. Prefer doing nothing conceptually over unnecessary novelty.",
        "source": pattern_summary(source),
        "controls": {
            "amount": params.amount,
            "density": params.density,
            "complexity": params.complexity,
            "humanize": params.humanize,
            "lockedTracks": params.locked_tracks,
        },
        "preferenceMemory": preference_memory,
    });

    let mut questions = BTreeMap::new();
    questions.insert(
        "operation".into(),
        json!({
            "type": "choice",
            "instructions": "Which transformation is the most musically useful next move for the current groove?",
            "criteria": {
                "complete": "Complete an empty or obviously incomplete groove by adding a coherent rhythmic foundation.",
                "variation": "Create a recognizable alternative while preserving the groove's identity.",
                "fill": "Add a short transition or turnaround, especially near the end of the phrase.",
                "humanize": "Keep note placement structurally intact while improving velocity and microtiming feel.",
                "simplify": "Remove excess activity and create more space without losing the pulse.",
                "complexify": "Add syncopation, percussion or detail while retaining coherence."
            }
        }),
    );
    questions.insert(
        "direction".into(),
        json!({
            "type": "choice",
            "instructions": "What energy direction should the next transformation support?",
            "criteria": {
                "hold": "Maintain the current energy and rhythmic identity.",
                "build": "Increase forward motion, density or anticipation.",
                "release": "Create more space, reduce pressure or settle the groove.",
                "break": "Create a transitional interruption or turnaround before returning."
            }
        }),
    );
    questions.insert(
        "intensity".into(),
        json!({
            "type": "choice",
            "instructions": "How strongly should the transformation alter the current groove?",
            "criteria": {
                "subtle": "Small, conservative changes that are immediately recognizable as the same groove.",
                "balanced": "Noticeable but controlled musical development.",
                "bold": "Strong variation appropriate for a new section or dramatic transition."
            }
        }),
    );
    questions.insert(
        "worthChanging".into(),
        json!({
            "type": "noul",
            "instructions": "Is it musically useful to transform the current groove now, given its structure and the user's historical preferences?",
            "criteria": {
                "true": "A transformation would likely improve usefulness, movement or expression.",
                "false": "The groove is already appropriate or the available transformations would add unnecessary change."
            }
        }),
    );

    SystemOneRequest {
        model: model.to_owned(),
        state,
        questions,
    }
}

fn parse_decision(
    response: SystemOneResponse,
    candidates: &[GrooveCandidate],
) -> Result<JevRerankDecision, JevError> {
    let choice = response
        .answers
        .get("bestCandidate")
        .ok_or_else(|| JevError::MissingAnswer("bestCandidate".into()))?;
    let selected_id = choice
        .get("choice")
        .and_then(Value::as_str)
        .ok_or_else(|| JevError::MissingAnswer("bestCandidate.choice".into()))?
        .to_owned();
    if !candidates
        .iter()
        .any(|candidate| candidate.id == selected_id)
    {
        return Err(JevError::UnknownCandidate(selected_id));
    }
    let confidence = choice
        .get("confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.0) as f32;
    let probabilities = choice
        .get("probabilities")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(key, value)| {
                    value
                        .as_f64()
                        .map(|probability| (key.clone(), probability as f32))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let safe_to_preview = response
        .answers
        .get("safeToPreview")
        .and_then(|answer| answer.get("noul"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0) as f32;

    Ok(JevRerankDecision {
        selected_id,
        confidence,
        probabilities,
        safe_to_preview,
        model: response.model,
        input_tokens: response.usage.input_tokens,
    })
}

fn parse_director_decision(response: SystemOneResponse) -> Result<JevDirectorDecision, JevError> {
    let (operation_choice, operation_confidence) =
        parse_choice_answer(&response.answers, "operation")?;
    let (direction_choice, direction_confidence) =
        parse_choice_answer(&response.answers, "direction")?;
    let (intensity_choice, intensity_confidence) =
        parse_choice_answer(&response.answers, "intensity")?;
    let worth_changing = response
        .answers
        .get("worthChanging")
        .and_then(|answer| answer.get("noul"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0) as f32;

    let operation = match operation_choice.as_str() {
        "complete" => GrooveOperation::Complete,
        "variation" => GrooveOperation::Variation,
        "fill" => GrooveOperation::Fill,
        "humanize" => GrooveOperation::Humanize,
        "simplify" => GrooveOperation::Simplify,
        "complexify" => GrooveOperation::Complexify,
        other => return Err(JevError::InvalidDirectorChoice(other.to_owned())),
    };
    let direction = match direction_choice.as_str() {
        "hold" => PerformanceDirection::Hold,
        "build" => PerformanceDirection::Build,
        "release" => PerformanceDirection::Release,
        "break" => PerformanceDirection::Break,
        other => return Err(JevError::InvalidDirectorChoice(other.to_owned())),
    };
    let intensity = match intensity_choice.as_str() {
        "subtle" => DirectorIntensity::Subtle,
        "balanced" => DirectorIntensity::Balanced,
        "bold" => DirectorIntensity::Bold,
        other => return Err(JevError::InvalidDirectorChoice(other.to_owned())),
    };

    Ok(JevDirectorDecision {
        operation,
        direction,
        intensity,
        confidence: operation_confidence
            .min(direction_confidence)
            .min(intensity_confidence),
        worth_changing,
        model: response.model,
        input_tokens: response.usage.input_tokens,
    })
}

fn parse_choice_answer(
    answers: &BTreeMap<String, Value>,
    key: &str,
) -> Result<(String, f32), JevError> {
    let answer = answers
        .get(key)
        .ok_or_else(|| JevError::MissingAnswer(key.to_owned()))?;
    let choice = answer
        .get("choice")
        .and_then(Value::as_str)
        .ok_or_else(|| JevError::MissingAnswer(format!("{key}.choice")))?
        .to_owned();
    let confidence = answer
        .get("confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.0) as f32;
    Ok((choice, confidence))
}

fn pattern_summary(pattern: &Pattern) -> Value {
    json!({
        "name": pattern.name,
        "bpm": pattern.bpm,
        "steps": pattern.total_steps,
        "swing": pattern.swing,
        "summary": compact_pattern_summary(pattern),
    })
}

fn compact_pattern_summary(pattern: &Pattern) -> Value {
    json!({
        "activeHits": pattern.tracks.iter().flat_map(|track| &track.steps).filter(|step| step.active).count(),
        "kick": active_positions(pattern, 0),
        "snare": active_positions(pattern, 1),
        "closedHat": active_positions(pattern, 2),
        "openHat": active_positions(pattern, 3),
        "clap": active_positions(pattern, 4),
        "percussion": (5..10).map(|track| active_positions(pattern, track)).collect::<Vec<_>>(),
    })
}

fn active_positions(pattern: &Pattern, track: usize) -> Vec<Value> {
    pattern
        .tracks
        .get(track)
        .map(|track| {
            track
                .steps
                .iter()
                .enumerate()
                .filter(|(_, step)| step.active)
                .map(|(index, step)| {
                    json!({
                        "step": index,
                        "velocity": step.velocity,
                        "microOffsetMicros": step.micro_offset_micros,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn candidate_guidance(metrics: &CandidateMetrics, operation: GrooveOperation) -> Value {
    json!({
        "operation": operation,
        "localHeuristicScore": metrics.local_score,
        "changedSteps": metrics.changed_steps,
        "activeHits": metrics.active_hits,
        "averageVelocity": metrics.average_velocity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mpd24_ai_groove_engine::{generate_candidates, GrooveParams};

    #[test]
    fn request_contains_choice_for_every_candidate() {
        let source = Pattern::demo();
        let params = GrooveParams::default();
        let candidates = generate_candidates(&source, GrooveOperation::Variation, &params, 4);
        let request = build_request(
            DEFAULT_MODEL,
            &source,
            &candidates,
            GrooveOperation::Variation,
            &params,
        );
        let criteria = request.questions["bestCandidate"]["criteria"]
            .as_object()
            .expect("criteria object");
        assert_eq!(criteria.len(), 4);
        assert!(criteria.contains_key("candidate-1"));
        assert_eq!(request.model, DEFAULT_MODEL);
    }

    #[test]
    fn parses_official_choice_and_noul_shapes() {
        let source = Pattern::demo();
        let params = GrooveParams::default();
        let candidates = generate_candidates(&source, GrooveOperation::Variation, &params, 2);
        let response = SystemOneResponse {
            model: "jev-1.13.0".into(),
            answers: BTreeMap::from([
                (
                    "bestCandidate".into(),
                    json!({
                        "type": "choice",
                        "choice": "candidate-2",
                        "confidence": 0.81,
                        "probabilities": {"candidate-1": 0.19, "candidate-2": 0.81}
                    }),
                ),
                (
                    "safeToPreview".into(),
                    json!({"type": "noul", "noul": 0.92}),
                ),
            ]),
            usage: Usage {
                input_tokens: 123,
                output_tokens: 0,
            },
        };
        let decision = parse_decision(response, &candidates).expect("decision");
        assert_eq!(decision.selected_id, "candidate-2");
        assert_eq!(decision.model, "jev-1.13.0");
        assert_eq!(decision.input_tokens, 123);
        assert!((decision.safe_to_preview - 0.92).abs() < f32::EPSILON);
    }

    #[test]
    fn parses_director_plan_from_typed_answers() {
        let response = SystemOneResponse {
            model: "jev-1.13.0".into(),
            answers: BTreeMap::from([
                (
                    "operation".into(),
                    json!({"type":"choice","choice":"variation","confidence":0.82}),
                ),
                (
                    "direction".into(),
                    json!({"type":"choice","choice":"build","confidence":0.76}),
                ),
                (
                    "intensity".into(),
                    json!({"type":"choice","choice":"bold","confidence":0.71}),
                ),
                ("worthChanging".into(), json!({"type":"noul","noul":0.88})),
            ]),
            usage: Usage {
                input_tokens: 222,
                output_tokens: 0,
            },
        };
        let plan = parse_director_decision(response).expect("director plan");
        assert_eq!(plan.operation, GrooveOperation::Variation);
        assert_eq!(plan.direction, PerformanceDirection::Build);
        assert_eq!(plan.intensity, DirectorIntensity::Bold);
        assert!((plan.confidence - 0.71).abs() < f32::EPSILON);
        assert!((plan.worth_changing - 0.88).abs() < f32::EPSILON);
    }

    #[test]
    fn director_request_contains_preference_memory() {
        let request = build_director_request(
            DEFAULT_MODEL,
            &Pattern::demo(),
            &GrooveParams::default(),
            json!({"total": 12, "accepted": 9}),
        );
        assert_eq!(request.state["preferenceMemory"]["total"], 12);
        assert!(request.questions.contains_key("operation"));
        assert!(request.questions.contains_key("worthChanging"));
    }
}
