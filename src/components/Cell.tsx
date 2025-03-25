import clsx from "clsx";
import { useEffect, useState } from "react";

export type CellState = "green" | "yellow" | "gray" | "empty";

type Props = {
	children?: React.ReactNode;
	state: CellState;
	changeOnClick?: boolean;
	cellIndex?: number;
	setCurrentGuessState?: React.Dispatch<React.SetStateAction<CellState[]>>;
};

function Cell({ state, children, changeOnClick, cellIndex, setCurrentGuessState }: Props) {
	const [currentState, setCurrentState] = useState<CellState>(state);
	useEffect(() => {
		if (children === "" || children === " ") {
			setCurrentState("empty");

			// Update the parent state if the props are provided
			if (setCurrentGuessState && cellIndex !== undefined) {
				setCurrentGuessState((prevState) => {
					const newState = [...prevState];
					newState[cellIndex] = "empty";
					return newState;
				});
			}
		}
	}, [children, setCurrentGuessState, cellIndex]);

	function onCellClicked() {
		if (!changeOnClick || children === "" || children === " ") return;

		const nextState: CellState =
			currentState === "green"
				? "yellow"
				: currentState === "yellow"
					? "gray"
					: currentState === "gray"
						? "empty"
						: "green";

		setCurrentState(nextState);

		// Update the parent state if the props are provided
		if (setCurrentGuessState && cellIndex !== undefined) {
			setCurrentGuessState((prevState) => {
				const newState = [...prevState];
				newState[cellIndex] = nextState;
				return newState;
			});
		}
	}

	return (
		<div
			onClick={onCellClicked}
			className={clsx(
				"grid size-14 place-items-center border border-gray-100 text-4xl font-bold text-white transition-colors duration-200 ease-in-out",
				changeOnClick && "cursor-pointer hover:scale-105",
				currentState == "empty" && "bg-transparent",
				currentState == "green" && "bg-[#528d4e]",
				currentState == "yellow" && "bg-[#b59f3b]",
				currentState == "gray" && "bg-[#3a3a3c]",
			)}
		>
			<span className="relative -top-[1px]">{children}</span>
		</div>
	);
}

export default Cell;
