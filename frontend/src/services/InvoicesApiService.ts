import {
  ApiPage,
  InvoiceCodifiedDTO,
  InvoiceListItemDTO,
  RequestInvoiceDTO,
  ResponseInvoiceDTO,
} from "@/models/ApiModel";
import { Invoice, InvoiceListItem } from "@/models/InvoiceModel";
import { formatISO, parseISO } from "date-fns";
import Vue from "vue";

export async function listInvoices(
  page: number,
  pageSize: number
): Promise<Array<InvoiceListItem>> {
  const response = await Vue.axios.get<ApiPage<InvoiceListItemDTO>>(
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
    codified: dto.codified,
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
    createdTimestamp: parseISO(dto.createdTimestamp),
    codifiedTimestamp: dto.codifiedTimestamp
      ? parseISO(dto.codifiedTimestamp)
      : undefined,
    invoiceDate: dto.invoiceDate ? parseISO(dto.invoiceDate) : undefined,
    subject: dto.subject,
    headerHTML: dto.headerHTML,
    footerHTML: dto.footerHTML,
    isCanceled: dto.isCanceled,
    isCancelation: dto.isCancelation,
  };
}

export async function fetchInvoice(id: number): Promise<Invoice> {
  const response = await Vue.axios.get<ResponseInvoiceDTO>(
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
  };
  return val;
}

export async function createInvoice(invoice: Invoice): Promise<Invoice> {
  const response = await Vue.axios.post(
    "/api/invoices",
    mapInvoiceToDTO(invoice)
  );
  return mapInvoiceDTOToInvoice(response.data);
}

export async function updateInvoice(invoice: Invoice): Promise<void> {
  await Vue.axios.put(`/api/invoices/${invoice.id}`, mapInvoiceToDTO(invoice));
}

export async function exportInvoice(invoice: Invoice): Promise<void> {
  await Vue.axios.post(`/api/invoices/${invoice.id}/export`);
}

export async function codifyInvoice(
  invoiceId: number
): Promise<InvoiceCodifiedDTO> {
  const response = await Vue.axios.post(`/api/invoices/${invoiceId}/codified`);
  return response.data;
}
