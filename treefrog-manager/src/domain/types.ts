// TypeScript domain mirrors for profiles — keep in sync with Rust/Python + JSON profiles
export type SystemEntry = {
  id: string;
  folder_aliases: string[];
  display_name?: string;
  core?: string | null;
  extensions: string[];
  archive_payload_valid?: string[];
  multi_file?: boolean;
};

export type ProfileSystems = {
  systems: SystemEntry[];
};

export type Kind = "rom" | "music" | "video" | "image" | "ebook" | "bios" | "lgpt_sample" | "lgpt_project" | "archive" | "unknown";
