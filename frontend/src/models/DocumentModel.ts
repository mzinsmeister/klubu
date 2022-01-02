export interface DocumentVersion {
  document: Document;
  version: number;
  createdTimestamp: Date;
}

export interface Document {
  id: number;
  lastVersion: number;
  mediaType: string;
}

export interface DocumentData {
  data: Uint8Array;
  mediaType: string;
}
