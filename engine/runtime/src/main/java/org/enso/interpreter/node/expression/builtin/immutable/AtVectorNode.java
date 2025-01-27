package org.enso.interpreter.node.expression.builtin.immutable;

import com.oracle.truffle.api.CompilerDirectives;
import com.oracle.truffle.api.interop.InteropLibrary;
import com.oracle.truffle.api.interop.InvalidArrayIndexException;
import com.oracle.truffle.api.interop.UnsupportedMessageException;
import com.oracle.truffle.api.nodes.Node;
import org.enso.interpreter.dsl.BuiltinMethod;
import org.enso.interpreter.node.expression.builtin.interop.syntax.HostValueToEnsoNode;
import org.enso.interpreter.runtime.EnsoContext;
import org.enso.interpreter.runtime.data.Vector;
import org.enso.interpreter.runtime.error.DataflowError;
import org.enso.interpreter.runtime.error.WarningsLibrary;

@BuiltinMethod(
    type = "Vector",
    name = "at",
    description = "Returns an element of Vector at the specified index.")
public class AtVectorNode extends Node {
  private @Child InteropLibrary interop = InteropLibrary.getFactory().createDispatched(3);
  private @Child WarningsLibrary warnings = WarningsLibrary.getFactory().createDispatched(3);
  private @Child HostValueToEnsoNode convert = HostValueToEnsoNode.build();

  Object execute(Vector self, long index) {
    try {
      return readElement(self, index);
    } catch (UnsupportedMessageException e) {
      CompilerDirectives.transferToInterpreter();
      throw new IllegalStateException(e);
    }
  }

  private Object readElement(Vector self, long index) throws UnsupportedMessageException {
    try {
      long actualIndex = index < 0 ? index + self.length(interop) : index;
      return self.readArrayElement(actualIndex, interop, warnings, convert);
    } catch (InvalidArrayIndexException e) {
      EnsoContext ctx = EnsoContext.get(this);
      return DataflowError.withoutTrace(
          ctx.getBuiltins().error().makeIndexOutOfBounds(index, self.length(interop)), this);
    }
  }
}
