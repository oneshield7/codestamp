import { toAppError } from '../api/http';
import {
  loadWsdl,
  wsdlSource,
  type SoapInterfaceInfo,
  type SoapOperationInfo,
  type WsdlDefinition,
} from '../api/soap';
import { RecentCache } from './recent';

/** WSDLs loaded this session, so reopening a SOAP card lists its operations again. */
const definitions = new RecentCache<WsdlDefinition>(20);

export interface FoundOperation {
  binding: SoapInterfaceInfo;
  operation: SoapOperationInfo;
}

/** The open SOAP card's WSDL: its bindings and operations once loaded. */
export class WsdlLoader {
  definition = $state<WsdlDefinition | null>(null);
  loading = $state(false);
  /** Why the WSDL could not be loaded. */
  error = $state<string | null>(null);

  /** Shows a WSDL loaded earlier from the same place, if any. */
  recall(where: string): void {
    this.definition = definitions.get(where.trim()) ?? null;
  }

  async load(where: string): Promise<WsdlDefinition | null> {
    if (this.loading || where.trim() === '') {
      return null;
    }
    this.loading = true;
    this.error = null;
    try {
      const definition = await loadWsdl(wsdlSource(where));
      this.definition = definition;
      definitions.set(where.trim(), definition);
      return definition;
    } catch (caught) {
      this.error = toAppError(caught).message;
      return null;
    } finally {
      this.loading = false;
    }
  }

  find(binding: string, operation: string): FoundOperation | null {
    const found = this.definition?.interfaces.find((item) => item.name === binding);
    const match = found?.operations.find((item) => item.name === operation);
    return found && match ? { binding: found, operation: match } : null;
  }
}
