import { useState, useCallback, useEffect } from "react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import type { SkillMeta, Skill } from "@/features/skill/types";
import * as skillService from "@/features/skill/services";

export function useSkills() {
  const { t } = useI18n();
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
      const message = err instanceof Error ? err.message : t.common.failedToLoad;
      setError(message);
      toast.error(message);
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
      const message = err instanceof Error ? err.message : t.common.failedToLoad;
      setError(message);
      toast.error(message);
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
    try {
      await skillService.createSkill(params);
      await load();
      toast.success(t.common.createdSuccessfully);
    } catch {
      toast.error(t.common.failedToCreate);
    }
  }, [load]);

  const update = useCallback(async (params: {
    name: string;
    description: string;
    category: string;
    content: string;
  }) => {
    try {
      await skillService.updateSkill(params);
      await load();
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    }
  }, [load]);

  const remove = useCallback(async (name: string) => {
    try {
      await skillService.deleteSkill(name);
      await load();
      toast.success(t.common.deletedSuccessfully);
    } catch {
      toast.error(t.common.failedToDelete);
    }
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
