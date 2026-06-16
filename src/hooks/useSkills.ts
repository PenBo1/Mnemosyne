import { useState, useCallback, useEffect } from "react";
import type { SkillMeta, Skill } from "@/types";
import * as skillService from "@/services/skill";

export function useSkills() {
  const [skills, setSkills] = useState<SkillMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterCategory, setFilterCategory] = useState("all");

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await skillService.listSkills();
      setSkills(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to load skills";
      setError(message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const refresh = useCallback(async () => {
    try {
      setLoading(true);
      await skillService.refreshSkills();
      await load();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to refresh skills";
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [load]);

  const getSkill = useCallback(async (name: string): Promise<Skill> => {
    return skillService.getSkill(name);
  }, []);

  const create = useCallback(async (params: {
    name: string;
    description: string;
    category: string;
    content: string;
  }) => {
    await skillService.createSkill(params);
    await load();
  }, [load]);

  const update = useCallback(async (params: {
    name: string;
    description: string;
    category: string;
    content: string;
  }) => {
    await skillService.updateSkill(params);
    await load();
  }, [load]);

  const remove = useCallback(async (name: string) => {
    await skillService.deleteSkill(name);
    await load();
  }, [load]);

  const filteredSkills = skills.filter(
    (skill) => filterCategory === "all" || skill.category === filterCategory
  );

  return {
    skills: filteredSkills,
    allSkills: skills,
    loading,
    error,
    filterCategory,
    setFilterCategory,
    refresh,
    getSkill,
    create,
    update,
    remove,
    reload: load,
  };
}
