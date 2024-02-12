// @ts-nocheck
// Use this file only for experimenting at run time
import * as React from "react";
interface Props {
  value: string;
}
export const ArrowExpressionComponent = (props: Props) => {
  return (
    <div>
      <span>
        <h2>Hello World</h2>
      </span>
    </div>
  );
};
export function FunctionDeclarationComponent(props: Props) {
  return (
    <div>
      <span>
        <h2>Hello World</h2>
      </span>
    </div>
  );
}
export const ArrowExpressionComponentReturnVariant = (props: Props) => (
  <div></div>
);
export const ArrowExpressionComponentNameSpaceVariant = (props: Props) => (
  <FunctionDeclarationComponent />
);
export const ArrowExpressionComponentFragmentVersion = (props: Props) => {
  return <></>;
};
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
ArrowExpressionComponent.displayName = "React_MOD_ArrowExpressionComponent";
ArrowExpressionComponentReturnVariant.displayName =
  "React_MOD_ArrowExpressionComponentReturnVariant";
ArrowExpressionComponentNameSpaceVariant.displayName =
  "React_MOD_ArrowExpressionComponentNameSpaceVariant";
ArrowExpressionComponentFragmentVersion.displayName =
  "React_MOD_ArrowExpressionComponentFragmentVersion";
ArrowExpressionComponentForwardRefWithMemberExpr.displayName =
  "React_MOD_ArrowExpressionComponentForwardRefWithMemberExpr";
ArrowExpressionComponentForwardRefWithoutMemberExpr.displayName =
  "React_MOD_ArrowExpressionComponentForwardRefWithoutMemberExpr";
ArrowExpressionComponentForwardRefWithFunctionExpression.displayName =
  "React_MOD_ArrowExpressionComponentForwardRefWithFunctionExpression";
FunctionDeclarationComponent.displayName =
  "React_MOD_FunctionDeclarationComponent";
