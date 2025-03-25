import clsx from "clsx";

export type CellState = "green" | "yellow" | "gray" | "empty";

type Props = {
	children?: React.ReactNode;
	state: CellState;
};

function Cell({ state, children }: Props) {
	return (
		<div
			className={clsx(
				"grid size-14 place-items-center border border-gray-100 text-4xl font-bold text-white",
				state == "empty" && "bg-transparent",
				state == "green" && "bg-[#528d4e]",
				state == "yellow" && "bg-[#b59f3b]",
				state == "gray" && "bg-[#3a3a3c]",
			)}
		>
			<span className="relative -top-[1px]">{children}</span>
		</div>
	);
}

export default Cell;
