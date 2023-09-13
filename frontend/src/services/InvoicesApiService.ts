import {
  type ApiPage,
  documentVerionFromDTO,
  type DocumentVersionDTO,
  type InvoiceCommittedDTO,
  type InvoiceListItemDTO,
  type RequestInvoiceDTO,
  type ResponseInvoiceDTO,
} from "@/models/ApiModel";
import type { DocumentVersion } from "@/models/DocumentModel";
import type { Invoice, InvoiceListItem } from "@/models/InvoiceModel";
import axios from "axios";
import { formatISO, parseISO } from "date-fns";

export async function listInvoices(
  page: number,
  pageSize: number
): Promise<Array<InvoiceListItem>> {
  const response = await axios.get<ApiPage<InvoiceListItemDTO>>(
    "/api/invoices",
    {
      params: {
        page: page,
        size: pageSize,
      },
    }
  );
  return response.data.content.map((dto) => ({
    id: dto.id,
    title: dto.title,
    createdTimestamp: parseISO(dto.createdTimestamp),
    customerContact: dto.customerContact,
    committed: dto.committed,
    isCancelation: dto.isCancelation,
    isCanceled: dto.isCanceled,
    invoiceNumber: dto.invoiceNumber,
    paidDate: dto.paidDate ? parseISO(dto.paidDate) : undefined,
  }));
}

function mapInvoiceDTOToInvoice(dto: ResponseInvoiceDTO): Invoice {
  return {
    id: dto.id,
    title: dto.title,
    customerContact: dto.customerContact,
    recipient: dto.recipient,
    items: dto.items,
    payments: dto.payments.map((payment) => ({
      amountCents: payment.amountCents,
      date: parseISO(payment.date),
    })),
    createdTimestamp: parseISO(dto.createdTimestamp),
    committedTimestamp: dto.committedTimestamp
      ? parseISO(dto.committedTimestamp)
      : undefined,
    invoiceDate: dto.invoiceDate ? parseISO(dto.invoiceDate) : undefined,
    subject: dto.subject,
    headerHTML: dto.headerHTML,
    footerHTML: dto.footerHTML,
    isCanceled: dto.isCanceled,
    isCancelation: dto.isCancelation,
    document: dto.document,
    invoiceNumber: dto.invoiceNumber,
  };
}

export async function fetchInvoice(id: number): Promise<Invoice> {
  const response = await axios.get<ResponseInvoiceDTO>(
    "/api/invoices/" + id
  );
  return mapInvoiceDTOToInvoice(response.data);
}

function mapInvoiceToDTO(invoice: Invoice): RequestInvoiceDTO {
  const val = {
    customerContactId: invoice.customerContact?.id,
    title: invoice.title,
    items: invoice.items,
    subject: invoice.subject,
    invoiceDate: invoice.invoiceDate
      ? formatISO(invoice.invoiceDate, { representation: "date" })
      : undefined,
    footerHTML: invoice.footerHTML,
    headerHTML: invoice.headerHTML,
    recipient: invoice.recipient,
    payments: invoice.payments.map((payment) => ({
      amountCents: payment.amountCents,
      date: formatISO(payment.date, { representation: "date" }),
    })),
  };
  return val;
}

export async function createInvoice(invoice: Invoice): Promise<Invoice> {
  const response = await axios.post(
    "/api/invoices",
    mapInvoiceToDTO(invoice)
  );
  return mapInvoiceDTOToInvoice(response.data);
}

export async function updateInvoice(invoice: Invoice): Promise<void> {
  await axios.put(`/api/invoices/${invoice.id}`, mapInvoiceToDTO(invoice));
}

export async function exportInvoice(
  invoice: Invoice
): Promise<DocumentVersion> {
  const response = await axios.post<DocumentVersionDTO>(
    `/api/invoices/${invoice.id}/export`
  );
  return documentVerionFromDTO(response.data);
}

export async function commitInvoice(
  invoiceId: number
): Promise<InvoiceCommittedDTO> {
  const response = await axios.post(`/api/invoices/${invoiceId}/committed`);
  return response.data;
}
