package dev.zinsmeister.klubu.exception

import org.springframework.http.HttpStatus
import org.springframework.web.bind.annotation.ResponseStatus

/**
 * This should be thrown inside the domain layer to signal that the resource
 * is in a state that doesn't allow the requested modification to be made.
 * It does specifically return an internal server error because in all cases where it is expected
 * that the user can try to modify a resource they are not allowed to modify this should be caught
 * and rethrown as an IllegalModificationException.
 *
 * I'm not sure whether this is the way to go but that's the way it is for now.
 */
@ResponseStatus(HttpStatus.INTERNAL_SERVER_ERROR)
class IllegalModificationException(msg: String): RuntimeException(msg)