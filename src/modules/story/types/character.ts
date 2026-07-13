// ── Character ─────────────────────────────────────────────

export interface Character {
  id: string;
  novel_id: string;
  name: string;
  role: string;
  age: string;
  gender: string;
  appearance: string;
  personality: string;
  backstory: string;
  motivation: string;
  fears: string;
  skills: string;
  description: string;
  traits: string[];
  custom_fields: string;
  created_at: string;
  updated_at: string;
}

export interface CharacterRelationship {
  id: string;
  novel_id: string;
  character_a_id: string;
  character_b_id: string;
  relationship_type: string;
  description: string;
  created_at: string;
}
