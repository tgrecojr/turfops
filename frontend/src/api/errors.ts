/**
 * Errors from the API client, and the one place that turns them into the sentence a
 * page shows. Every message the owner sees says what went wrong and, where there is one,
 * what to do: never raw JSON, a bare status code or internal detail.
 */

/** A failed request: the HTTP status (0 when no response arrived) and a message. */
export class ApiError extends Error {
	readonly status: number;
	/** The request was cancelled by the client's timeout. */
	readonly timedOut: boolean;

	constructor(status: number, message: string, timedOut = false) {
		super(message);
		this.name = "ApiError";
		this.status = status;
		this.timedOut = timedOut;
	}
}

const SERVER_ERROR =
	"Something went wrong on the server. Try again; the details are in the server log.";
const TIMED_OUT = "That took too long to answer. Try again.";
const UNREACHABLE =
	"TurfOps couldn't be reached. Check your connection and try again.";
const NO_PROFILE =
	"There is no lawn profile yet. Set one up in Settings to get started.";

/** The plain sentence for a status when the server sent no usable message. */
export function sentenceForStatus(status: number): string {
	if (status >= 500) return SERVER_ERROR;
	if (status === 404) return "That wasn't found.";
	if (status === 413) return "That request is too large to accept.";
	return "The request couldn't be completed. Try again.";
}

/**
 * The sentence to show the owner for a failed request or a thrown error.
 * @spec UI-ERR-005, UI-ERR-006, UI-ERR-007, UI-ERR-009, UI-ERR-010, UI-SHELL-005
 */
export function describeError(err: unknown): string {
	if (err instanceof ApiError) {
		if (err.timedOut) return TIMED_OUT;
		if (err.status === 0) return UNREACHABLE;
		if (err.status === 404 && /lawn profile/i.test(err.message))
			return NO_PROFILE;
		if (err.status >= 500 && err.status !== 503) return SERVER_ERROR;
		return err.message || sentenceForStatus(err.status);
	}
	// fetch rejects with a TypeError when no response arrives at all.
	if (err instanceof TypeError) return UNREACHABLE;
	return SERVER_ERROR;
}
