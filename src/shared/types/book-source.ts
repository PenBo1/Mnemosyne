// ── Book Source (Novel Download) ──────────────────────

export interface BookSource {
  url: string;
  name: string;
  comment: string;
  disabled: boolean;
  search?: SearchRule;
  book?: BookRule;
  toc?: TocRule;
  chapter?: ChapterRule;
}

export interface SearchRule {
  disabled: boolean;
  url: string;
  method: string;
  data: string;
  cookies: string;
  result: string;
  book_name: string;
  author: string;
  category: string;
  word_count: string;
  status: string;
  latest_chapter: string;
  last_update_time: string;
  pagination: boolean;
  next_page: string;
}

export interface BookRule {
  url: string;
  book_name: string;
  author: string;
  intro: string;
  category: string;
  cover_url: string;
  latest_chapter: string;
  last_update_time: string;
  status: string;
}

export interface TocRule {
  base_uri: string;
  url: string;
  item: string;
  is_desc: boolean;
  pagination: boolean;
  next_page: string;
}

export interface ChapterRule {
  title: string;
  content: string;
  paragraph_tag_closed: boolean;
  paragraph_tag: string;
  filter_txt: string;
  filter_tag: string;
  pagination: boolean;
  next_page: string;
}

export interface SearchBookResult {
  book_name: string;
  author: string;
  url: string;
  category: string;
  word_count: string;
  status: string;
  latest_chapter: string;
  last_update_time: string;
  source_name: string;
  source_url: string;
}
