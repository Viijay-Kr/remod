// @ts-nocheck
import * as React from "react";
interface Props {
  value: string;
}
export const ArrowExpressionComponent = (props: Props) => {
  const renderComponent = () => {
    return <span>Foo Bar</span>;
  };
  return (
    <div>
      <span>
        <h2>Hello World</h2>
      </span>

      {renderComponent()}
    </div>
  );
};
ArrowExpressionComponent.displayName = "React_MOD_ArrowExpressionComponent";
