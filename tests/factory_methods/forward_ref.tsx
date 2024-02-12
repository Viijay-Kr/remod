// @ts-nocheck
import * as React from "react";

interface Props {
  value: string;
}

export const ArrowExpressionComponentForwardRefWithMemberExpr =
  React.forwardRef((props: Props, ref: unknown) => {
    return <div></div>;
  });

export const ArrowExpressionComponentForwardRefWithoutMemberExpr = forwardRef(
  (props: Props, ref: unknown) => {
    return <div></div>;
  }
);

export const ArrowExpressionComponentForwardRefWithFunctionExpression =
  forwardRef(function (props: Props, ref: unknown) {
    return <div></div>;
});
