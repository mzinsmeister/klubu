import { Contact } from "./ContactModel";
import { Money, Recipient } from "./CommonModel";
import { Item } from "./CommonModel";
import { Document, DocumentVersion } from "./DocumentModel";
import { parseISO } from "date-fns";
import { ReceiptItem } from "./ReceiptModel";

export interface ApiPage<T> {
  content: Array<T>;
}

export interface RequestOfferDTO {
  title?: string;
  customerContactId?: number;
  items: Array<Item>;
  recipient?: Recipient;
  offerDate?: string;
  validUntilDate?: string;
  subject?: string;
  headerHTML?: string;
  footerHTML?: string;
}

export interface ResponseOfferDTO {
  id: number;
  revision: number;
  title?: string;
  customerContact?: Contact;
  recipient?: Recipient;
  items: Array<Item>;
  createdTimestamp: string;
  committedTimestamp?: string;
  offerDate?: string;
  validUntilDate?: string;
  subject?: string;
  headerHTML?: string;
  footerHTML?: string;
  document?: Document;
}

export interface OfferCommittedDTO {
  committedTimestamp: string;
}

export interface OfferListItemDTO {
  id: number;
  revision: number;
  title?: string;
  createdTimestamp: string;
  customerContact: Contact;
}

export interface OfferRevisionListDTO {
  revisions: Array<OfferRevisionListItemDTO>;
}

export interface OfferRevisionListItemDTO {
  revisionNumber: number;
  createdTimestamp: string;
}

export interface ResponseInvoiceDTO {
  id: number;
  items: Array<Item>;
  createdTimestamp: string;
  committedTimestamp?: string;
  invoiceNumber?: number;
  paidDate?: string;
  invoiceDate?: string;
  isCanceled: boolean;
  isCancelation: boolean;
  correctedInvoiceId?: number;
  customerContact?: Contact;
  document?: Document;
  recipient?: Recipient;
  headerHTML?: string;
  footerHTML?: string;
  title?: string;
  subject?: string;
}

export interface RequestInvoiceDTO {
  items: Array<Item>;
  customerContactId?: number;
  paidDate?: string;
  invoiceDate?: string;
  recipient?: Recipient;
  headerHTML?: string;
  footerHTML?: string;
  title?: string;
  subject?: string;
}

export interface InvoiceListItemDTO {
  id: number;
  title?: string;
  createdTimestamp: string;
  customerContact?: Contact;
  paidDate?: string;
  committed: boolean;
  invoiceNumber?: number;
  isCanceled: boolean;
  isCancelation: boolean;
}

export interface InvoiceCommittedDTO {
  invoiceNumber: number;
  committedTimestamp: string;
}

export interface DocumentVersionDTO {
  document: Document;
  version: number;
  createdTimestamp: string;
}

export function documentVerionFromDTO(
  dto: DocumentVersionDTO
): DocumentVersion {
  return {
    version: dto.version,
    createdTimestamp: parseISO(dto.createdTimestamp),
    document: dto.document,
  };
}

export interface ReceiptListItemDTO {
  id: number;
  createdTimestamp: string;
  supplierContact?: Contact;
  paidDate?: string;
  dueDate?: string;
  receiptDate?: string;
  committed: boolean;
  receiptNumber?: string;
}

export interface ResponseReceiptDTO {
  id: number;
  items: Array<ReceiptItem>;
  createdTimestamp: string;
  committedTimestamp?: string;
  receiptNumber?: string;
  paidDate?: string;
  receiptDate?: string;
  dueDate?: string;
  supplierContact?: Contact;
  document?: Document;
}

export interface RequestReceiptDTO {
  receiptNumber: string;
  items: Array<ReceiptItem>;
  supplierContactId?: number;
  paidDate?: string;
  receiptDate?: string;
  dueDate?: string;
  document?: RequestReceiptDocumentDTO;
}

export interface RequestReceiptItemDTO {
  item: string;
  price: Money;
  categoryId: number
}

export interface RequestReceiptDocumentDTO {
  data: string;
  mediaType: string;
}

export interface ReceiptCommittedDTO {
  committedTimestamp: string;
}
